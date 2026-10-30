use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};
use crate::integrations::openmaus::{get_openmaus_data_dir, load_usage_report_from, UsageItem, UsageReport};
use crate::integrations::openmaus_activity::{ActivityEvent, ActivityMonitor, ActivitySnapshot};

pub const EXPANDED_SIZE:(f32,f32)=(560.0,232.0);
pub const PAGE_SIZE:usize=2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenMausTab { Activity, Schedule, Usage }

struct Update { activity:ActivitySnapshot, report:Option<UsageReport>, notifications:Vec<ActivityEvent> }

pub struct OpenMausSection {
    pub report:UsageReport,
    pub activity:ActivitySnapshot,
    pub tab:OpenMausTab,
    pub usage_group:usize,
    pub offset:usize,
    pub notifications_enabled:bool,
    pub unread:usize,
    pending:VecDeque<ActivityEvent>,
    updates:Receiver<Update>,
    refresh:SyncSender<()>,
}

impl OpenMausSection {
    pub fn new()->Self { Self::from_directory(get_openmaus_data_dir()) }

    pub fn from_directory(directory:PathBuf)->Self {
        let (tx,updates)=mpsc::sync_channel(2);
        let (refresh,commands)=mpsc::sync_channel(1);
        std::thread::spawn(move|| {
            let mut monitor=ActivityMonitor::new(directory.clone());
            let mut last_usage=Instant::now()-Duration::from_secs(10);
            loop {
                let forced=commands.try_iter().count()>0;
                let poll=monitor.poll();
                let report=if forced || last_usage.elapsed()>=Duration::from_secs(5) {
                    last_usage=Instant::now(); Some(load_usage_report_from(&directory))
                }else{None};
                if tx.send(Update{activity:poll.snapshot,report,notifications:poll.notifications}).is_err(){break;}
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        Self {report:UsageReport::default(),activity:ActivitySnapshot::default(),tab:OpenMausTab::Activity,
            usage_group:0,offset:0,notifications_enabled:true,unread:0,pending:VecDeque::new(),updates,refresh}
    }

    pub fn refresh_if_needed(&mut self) {
        while let Ok(update)=self.updates.try_recv() {
            self.activity=update.activity;
            if let Some(report)=update.report {self.report=report;}
            self.unread=self.unread.saturating_add(update.notifications.len());
            if self.notifications_enabled {
                for event in update.notifications.into_iter().rev() {
                    if event.kind!=crate::integrations::openmaus_activity::ActivityKind::AgentStarted {self.pending.push_back(event);}
                }
                while self.pending.len()>12 {self.pending.pop_front();}
            }
        }
        self.offset=self.offset.min(self.row_count().saturating_sub(1)/PAGE_SIZE*PAGE_SIZE);
    }

    pub fn request_refresh(&self) { let _=self.refresh.try_send(()); }
    pub fn set_notifications(&mut self,enabled:bool) {
        self.notifications_enabled=enabled;
        if !enabled {self.pending.clear();}
    }
    pub fn next_notification(&mut self)->Option<ActivityEvent> {
        let now=crate::integrations::openmaus_activity::timestamp_ms();
        while let Some(event)=self.pending.pop_front() {
            if now.saturating_sub(event.at_ms)<=120_000 {return Some(event);}
        }
        None
    }
    pub fn mark_read(&mut self) {self.unread=0;self.pending.clear();}
    pub fn set_tab(&mut self,tab:OpenMausTab) {self.tab=tab;self.offset=0;if tab==OpenMausTab::Activity {self.mark_read();}}
    pub fn usage_items(&self)->&[UsageItem] {
        match self.usage_group {1=>&self.report.models,2=>&self.report.providers,_=>&self.report.bots}
    }
    pub fn row_count(&self)->usize {
        match self.tab {OpenMausTab::Activity=>self.activity.events.len(),OpenMausTab::Schedule=>self.activity.routines.len(),OpenMausTab::Usage=>self.usage_items().len()}
    }
    pub fn set_page(&mut self,offset:usize) {self.offset=(offset/PAGE_SIZE).min(self.row_count().saturating_sub(1)/PAGE_SIZE)*PAGE_SIZE;}
}
