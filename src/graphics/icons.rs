use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use crate::graphics::d2d::D2DContext;
use crate::integrations::system_info::SystemStatus;

#[derive(Clone, Copy)]
pub enum Icon { Play, Pause, Previous, Next, Lyrics, Reset, Drop, Sliders, Bell, Plus,
    Pencil, Trash, Target, Coffee, Moon, Folder, File, Image, Music, Clock, Tasks, Bot, Chip, Chart, Plug }

pub fn draw(d:&D2DContext, icon:Icon, cx:f32, cy:f32, size:f32, b:&ID2D1SolidColorBrush) {
    let s=size/20.0;
    let line=|x0:f32,y0:f32,x1:f32,y1:f32| d.draw_line(cx+x0*s,cy+y0*s,cx+x1*s,cy+y1*s,b,1.65*s,true);
    let rect=|x:f32,y:f32,w:f32,h:f32|D2D_RECT_F{left:cx+x*s,top:cy+y*s,right:cx+(x+w)*s,bottom:cy+(y+h)*s};
    let poly=|points:&[(f32,f32)]| { let p:Vec<_>=points.iter().map(|(x,y)|(cx+x*s,cy+y*s)).collect(); d.fill_polygon(&p,b); };
    match icon {
        Icon::Trash=>{line(-7.0,-5.0,7.0,-5.0);line(-3.0,-8.0,3.0,-8.0);
            line(-5.0,-5.0,-4.0,8.0);line(-4.0,8.0,4.0,8.0);line(4.0,8.0,5.0,-5.0);
            line(-1.5,-1.0,-1.5,5.0);line(1.5,-1.0,1.5,5.0);},
        Icon::Play=>poly(&[(-4.0,-7.0),(7.0,0.0),(-4.0,7.0)]),
        Icon::Pause=>{d.fill_rounded_rect(rect(-6.0,-7.0,4.0,14.0),s,s,b);d.fill_rounded_rect(rect(2.0,-7.0,4.0,14.0),s,s,b);},
        Icon::Previous|Icon::Next=>{let side=if matches!(icon,Icon::Next){1.0}else{-1.0};
            poly(&[(-4.0*side,-6.0),(4.0*side,0.0),(-4.0*side,6.0)]);line(7.0*side,-6.0,7.0*side,6.0);},
        Icon::Lyrics=>{d.draw_rounded_rect(rect(-8.0,-6.0,16.0,12.0),3.0*s,3.0*s,b,1.5*s);
            line(-4.0,6.0,-5.0,9.0);line(-5.0,9.0,0.0,6.0);
            d.fill_ellipse(cx-3.0*s,cy,1.1*s,1.1*s,b);d.fill_ellipse(cx+3.0*s,cy,1.1*s,1.1*s,b);},
        Icon::Reset=>{d.draw_arc_segment(cx,cy,6.5*s,-160.0,140.0,1.7*s,b);
            line(-7.0,-6.0,-7.0,-1.0);line(-7.0,-1.0,-2.0,-1.0);},
        Icon::Drop=>d.fill_droplet(cx,cy,8.0*s,b),
        Icon::Sliders=>{for(y,x)in[(-5.0,-3.0),(0.0,4.0),(5.0,-1.0)]{line(-8.0,y,8.0,y);d.fill_ellipse(cx+x*s,cy+y*s,2.2*s,2.2*s,b);}},
        Icon::Bell=>{line(-6.0,4.0,-5.0,-3.0);line(5.0,-3.0,6.0,4.0);line(-6.0,4.0,6.0,4.0);
            d.draw_arc_segment(cx,cy-2.0*s,5.0*s,180.0,360.0,1.6*s,b);line(0.0,-8.0,0.0,-7.0);d.fill_ellipse(cx,cy+7.0*s,1.7*s,1.2*s,b);},
        Icon::Plus=>{line(-6.0,0.0,6.0,0.0);line(0.0,-6.0,0.0,6.0);},
        Icon::Pencil=>{poly(&[(-6.0,3.0),(3.0,-6.0),(6.0,-3.0),(-3.0,6.0)]);poly(&[(-7.0,7.0),(-6.0,3.0),(-3.0,6.0)]);},
        Icon::Target=>{d.draw_ellipse(cx,cy,7.0*s,7.0*s,b,1.6*s);d.draw_ellipse(cx,cy,3.2*s,3.2*s,b,1.3*s);line(0.0,0.0,7.0,-7.0);},
        Icon::Coffee=>{d.draw_rounded_rect(rect(-7.0,-2.0,12.0,9.0),2.0*s,2.0*s,b,1.5*s);
            line(5.0,-1.0,8.0,-1.0);line(8.0,-1.0,8.0,4.0);line(8.0,4.0,5.0,4.0);line(-4.0,-7.0,-4.0,-4.0);line(1.0,-7.0,1.0,-4.0);},
        Icon::Moon=>d.draw_arc_segment(cx,cy,5.8*s,-145.0,145.0,1.7*s,b),
        Icon::Folder=>{d.draw_rounded_rect(rect(-8.0,-4.0,16.0,11.0),2.0*s,2.0*s,b,1.6*s);line(-8.0,-4.0,-8.0,-7.0);line(-8.0,-7.0,-2.0,-7.0);line(-2.0,-7.0,1.0,-4.0);},
        Icon::File|Icon::Tasks=>{d.draw_rounded_rect(rect(-6.5,-8.0,13.0,16.0),2.0*s,2.0*s,b,1.5*s);
            line(-3.5,-3.0,3.5,-3.0);line(-3.5,1.0,3.5,1.0);line(-3.5,5.0,1.5,5.0);},
        Icon::Image=>{d.draw_rounded_rect(rect(-8.0,-7.0,16.0,14.0),2.0*s,2.0*s,b,1.4*s);
            line(-6.0,5.0,-1.0,-1.0);line(-1.0,-1.0,3.0,3.0);line(3.0,3.0,6.0,0.0);d.fill_ellipse(cx+4.0*s,cy-3.0*s,1.6*s,1.6*s,b);},
        Icon::Music=>{line(-3.0,-6.0,-3.0,5.0);line(-3.0,-6.0,6.0,-8.0);line(6.0,-8.0,6.0,3.0);
            d.fill_ellipse(cx-5.0*s,cy+5.0*s,3.0*s,2.2*s,b);d.fill_ellipse(cx+4.0*s,cy+3.0*s,3.0*s,2.2*s,b);},
        Icon::Clock=>{d.draw_ellipse(cx,cy+1.0*s,6.5*s,6.5*s,b,1.6*s);line(0.0,1.0,0.0,-3.5);line(0.0,1.0,3.0,3.0);line(-3.0,-9.0,3.0,-9.0);line(0.0,-9.0,0.0,-6.0);},
        Icon::Bot=>{d.draw_rounded_rect(rect(-8.0,-5.0,16.0,12.0),3.0*s,3.0*s,b,1.5*s);
            line(0.0,-8.0,0.0,-5.0);d.fill_ellipse(cx,cy-9.0*s,1.3*s,1.3*s,b);
            d.fill_ellipse(cx-3.5*s,cy,1.4*s,1.4*s,b);d.fill_ellipse(cx+3.5*s,cy,1.4*s,1.4*s,b);line(-3.0,4.0,3.0,4.0);},
        Icon::Chip=>{d.draw_rounded_rect(rect(-5.5,-5.5,11.0,11.0),1.7*s,1.7*s,b,1.5*s);
            for k in [-3.0,3.0]{line(k,-8.0,k,-5.5);line(k,5.5,k,8.0);line(-8.0,k,-5.5,k);line(5.5,k,8.0,k);}},
        Icon::Chart=>{line(-8.0,-7.0,-8.0,7.0);line(-8.0,7.0,8.0,7.0);line(-5.0,3.0,-1.0,-1.0);line(-1.0,-1.0,3.0,1.0);line(3.0,1.0,7.0,-6.0);},
        Icon::Plug=>{line(-4.0,-8.0,-4.0,-3.0);line(4.0,-8.0,4.0,-3.0);
            d.draw_rounded_rect(rect(-6.0,-3.0,12.0,7.0),3.0*s,3.0*s,b,1.6*s);line(0.0,4.0,0.0,9.0);},
    }
}

pub fn battery_fraction(status:&SystemStatus)->Option<f32> {
    if status.is_battery_present {status.battery_percent.map(|p|p.min(100) as f32/100.0)} else {None}
}

pub fn power(d:&D2DContext,status:&SystemStatus,cx:f32,cy:f32,size:f32,fg:&ID2D1SolidColorBrush,muted:&ID2D1SolidColorBrush) {
    if !status.is_battery_present {draw(d,Icon::Plug,cx,cy,size,if status.is_online{fg}else{muted});return;}
    let s=size/20.0;
    let r=D2D_RECT_F{left:cx-10.0*s,top:cy-5.5*s,right:cx+8.0*s,bottom:cy+5.5*s};
    d.draw_rounded_rect(r,2.0*s,2.0*s,fg,1.5*s);
    d.fill_rounded_rect(D2D_RECT_F{left:cx+9.0*s,top:cy-2.0*s,right:cx+11.0*s,bottom:cy+2.0*s},s,s,fg);
    if let Some(amount)=battery_fraction(status) {
        if amount>0.0 {d.fill_rounded_rect(D2D_RECT_F{left:r.left+2.5*s,top:r.top+2.5*s,right:r.left+2.5*s+13.0*s*amount,bottom:r.bottom-2.5*s},s,s,fg);}
    } else {for x in [-5.0,0.0,5.0]{d.fill_ellipse(cx+x*s,cy,0.8*s,0.8*s,muted);}}
    if status.is_charging {draw(d,Icon::Plug,cx,cy+12.0*s,size*0.35,fg);}
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn battery_capacity_is_only_drawn_when_reported() {
        let mut s=SystemStatus::default();assert_eq!(battery_fraction(&s),None);
        s.is_battery_present=true;assert_eq!(battery_fraction(&s),None);
        s.battery_percent=Some(72);assert_eq!(battery_fraction(&s),Some(0.72));
    }
}
