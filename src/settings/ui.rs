use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1HwndRenderTarget, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{IDWriteTextFormat, DWRITE_MEASURING_MODE_NATURAL};

use crate::config::AppSettings;
use crate::graphics::dwrite::DWriteContext;

pub const COLOR_BG: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.08, g: 0.08, b: 0.09, a: 1.0 }; // #141417
pub const COLOR_SIDEBAR: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.095, g: 0.095, b: 0.105, a: 1.0 }; // #18181b
pub const COLOR_CARD_BG: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.125, g: 0.125, b: 0.14, a: 1.0 };
pub const COLOR_CARD_BORDER: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.19, g: 0.19, b: 0.21, a: 1.0 };
pub const COLOR_CARD_HOVER: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.16, g: 0.16, b: 0.18, a: 1.0 };
pub const COLOR_ACCENT: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.23, g: 0.51, b: 0.96, a: 1.0 }; // #3b82f6
pub const COLOR_ACCENT_HOVER: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.30, g: 0.58, b: 1.0, a: 1.0 };
pub const COLOR_TEXT_PRIMARY: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.96, g: 0.96, b: 0.96, a: 1.0 };
pub const COLOR_TEXT_SECONDARY: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.63, g: 0.63, b: 0.67, a: 1.0 };
pub const COLOR_TEXT_MUTED: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.44, g: 0.44, b: 0.48, a: 1.0 };
pub const COLOR_SWITCH_OFF: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.22, g: 0.22, b: 0.24, a: 1.0 };
pub const COLOR_SWITCH_KNOB: D2D1_COLOR_F = D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
pub const COLOR_SIDEBAR_ACTIVE: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.17, g: 0.17, b: 0.20, a: 1.0 };
pub const COLOR_SIDEBAR_HOVER: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.13, g: 0.13, b: 0.15, a: 1.0 };
pub const COLOR_DANGER: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.90, g: 0.25, b: 0.25, a: 1.0 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsAction {
    SelectTab(usize),
    ToggleAutostart,
    ToggleMascot,
    ToggleMinimalNotch,
    ToggleAutoMinimizeOnMaximize,
    ToggleSectionMusic,
    ToggleSectionFocus,
    ToggleSectionWater,
    ToggleSectionShelf,
    ToggleSectionTodo,
    ToggleSectionOpenMaus,
    ToggleSectionAiUsage,
    ToggleSectionSaham,
    ToggleAiAntigravity,
    ToggleAiClaude,
    ToggleAiCodex,
    ToggleAiOpenCode,
    SetAiDefaultProvider(u32),
    AdjustAiPoll(i32),
    ToggleAiProportionBar,
    ToggleMusicGlow,
    ToggleDynamicLyricsWidth,
    ToggleDiscordRpc,
    AdjustFocusWorkMins(i32),
    AdjustFocusBreakMins(i32),
    AdjustWaterIntervalMins(i32),
    AdjustWaterTargetGlasses(i32),
    SetAccentMode(u32),
    AdjustAcrylicOpacity(f32),
    ResetToDefaults,
}

pub struct ClickRegion {
    pub rect: D2D_RECT_F,
    pub action: SettingsAction,
}

pub struct SettingsRenderer;

impl SettingsRenderer {
    pub fn draw_text_str(
        target: &ID2D1HwndRenderTarget,
        text: &str,
        format: &IDWriteTextFormat,
        rect: &D2D_RECT_F,
        brush: &ID2D1SolidColorBrush,
    ) {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            target.DrawText(
                &utf16,
                format,
                rect,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    pub fn draw_toggle(
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        x: f32,
        y: f32,
        active: bool,
    ) {
        let w = 42.0f32;
        let h = 22.0f32;
        let radius = 11.0f32;

        let bg_color = if active { COLOR_ACCENT } else { COLOR_SWITCH_OFF };
        unsafe {
            brush.SetColor(&bg_color);
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: x,
                        top: y,
                        right: x + w,
                        bottom: y + h,
                    },
                    radiusX: radius,
                    radiusY: radius,
                },
                brush,
            );

            // Knob
            brush.SetColor(&COLOR_SWITCH_KNOB);
            let knob_x = if active { x + w - radius - 2.0 } else { x + radius + 2.0 };
            let knob_y = y + h * 0.5;
            let knob_r = radius - 3.0;
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: knob_x - knob_r,
                        top: knob_y - knob_r,
                        right: knob_x + knob_r,
                        bottom: knob_y + knob_r,
                    },
                    radiusX: knob_r,
                    radiusY: knob_r,
                },
                brush,
            );
        }
    }

    pub fn draw_stepper(
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        dwrite: &DWriteContext,
        x: f32,
        y: f32,
        value_str: &str,
        dec_action: SettingsAction,
        inc_action: SettingsAction,
        regions: &mut Vec<ClickRegion>,
    ) {
        let btn_w = 26.0f32;
        let h = 26.0f32;
        let val_w = 54.0f32;

        unsafe {
            // Minus button
            let minus_rect = D2D_RECT_F {
                left: x,
                top: y,
                right: x + btn_w,
                bottom: y + h,
            };
            brush.SetColor(&COLOR_CARD_HOVER);
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: minus_rect,
                    radiusX: 5.0,
                    radiusY: 5.0,
                },
                brush,
            );
            brush.SetColor(&COLOR_TEXT_PRIMARY);
            Self::draw_text_str(target, "-", &dwrite.format_center, &minus_rect, brush);
            regions.push(ClickRegion {
                rect: minus_rect,
                action: dec_action,
            });

            // Value text
            let val_rect = D2D_RECT_F {
                left: x + btn_w,
                top: y,
                right: x + btn_w + val_w,
                bottom: y + h,
            };
            Self::draw_text_str(target, value_str, &dwrite.format_center, &val_rect, brush);

            // Plus button
            let plus_rect = D2D_RECT_F {
                left: x + btn_w + val_w,
                top: y,
                right: x + btn_w * 2.0 + val_w,
                bottom: y + h,
            };
            brush.SetColor(&COLOR_CARD_HOVER);
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: plus_rect,
                    radiusX: 5.0,
                    radiusY: 5.0,
                },
                brush,
            );
            brush.SetColor(&COLOR_TEXT_PRIMARY);
            Self::draw_text_str(target, "+", &dwrite.format_center, &plus_rect, brush);
            regions.push(ClickRegion {
                rect: plus_rect,
                action: inc_action,
            });
        }
    }

    pub fn render(
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        dwrite: &DWriteContext,
        settings: &AppSettings,
        active_tab: usize,
        mouse_pos: (f32, f32),
        width: f32,
        height: f32,
    ) -> Vec<ClickRegion> {
        let mut regions = Vec::new();

        unsafe {
            // Background
            brush.SetColor(&COLOR_BG);
            target.FillRectangle(
                &D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: width,
                    bottom: height,
                },
                brush,
            );

            // Left Sidebar
            let sidebar_w = 200.0f32;
            brush.SetColor(&COLOR_SIDEBAR);
            target.FillRectangle(
                &D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: sidebar_w,
                    bottom: height,
                },
                brush,
            );

            // Divider line
            brush.SetColor(&COLOR_CARD_BORDER);
            target.DrawLine(
                D2D_POINT_2F { x: sidebar_w, y: 0.0 },
                D2D_POINT_2F { x: sidebar_w, y: height },
                brush,
                1.0,
                None,
            );

            // App branding in sidebar
            brush.SetColor(&COLOR_TEXT_PRIMARY);
            Self::draw_text_str(
                target,
                "Nocan Living Notch",
                &dwrite.format_regular_bold,
                &D2D_RECT_F {
                    left: 18.0,
                    top: 20.0,
                    right: sidebar_w - 18.0,
                    bottom: 42.0,
                },
                brush,
            );

            brush.SetColor(&COLOR_TEXT_MUTED);
            Self::draw_text_str(
                target,
                "Pengaturan & Preferensi",
                &dwrite.format_small,
                &D2D_RECT_F {
                    left: 18.0,
                    top: 38.0,
                    right: sidebar_w - 18.0,
                    bottom: 58.0,
                },
                brush,
            );

            // Tabs in Sidebar
            let tab_labels = [
                ("Umum", "⚙️"),
                ("Seksi Notch", "🎛️"),
                ("AI Monitor", "🤖"),
                ("Musik & Lirik", "🎵"),
                ("Produktivitas", "⏱️"),
                ("Tampilan", "🎨"),
                ("Tentang Nocan", "ℹ️"),
            ];

            let tab_y_start = 72.0f32;
            let tab_h = 38.0f32;
            let tab_spacing = 4.0f32;

            for (idx, (label, icon)) in tab_labels.iter().enumerate() {
                let tab_y = tab_y_start + idx as f32 * (tab_h + tab_spacing);
                let tab_rect = D2D_RECT_F {
                    left: 12.0,
                    top: tab_y,
                    right: sidebar_w - 12.0,
                    bottom: tab_y + tab_h,
                };

                let is_hover = mouse_pos.0 >= tab_rect.left
                    && mouse_pos.0 <= tab_rect.right
                    && mouse_pos.1 >= tab_rect.top
                    && mouse_pos.1 <= tab_rect.bottom;

                if idx == active_tab {
                    brush.SetColor(&COLOR_SIDEBAR_ACTIVE);
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: tab_rect,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        brush,
                    );
                    // Blue active indicator pill on left
                    brush.SetColor(&COLOR_ACCENT);
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: D2D_RECT_F {
                                left: tab_rect.left + 2.0,
                                top: tab_rect.top + 8.0,
                                right: tab_rect.left + 5.0,
                                bottom: tab_rect.bottom - 8.0,
                            },
                            radiusX: 1.5,
                            radiusY: 1.5,
                        },
                        brush,
                    );
                } else if is_hover {
                    brush.SetColor(&COLOR_SIDEBAR_HOVER);
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: tab_rect,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        brush,
                    );
                }

                // Text
                let text_color = if idx == active_tab {
                    COLOR_TEXT_PRIMARY
                } else if is_hover {
                    COLOR_TEXT_SECONDARY
                } else {
                    COLOR_TEXT_MUTED
                };
                brush.SetColor(&text_color);

                let display_text = format!("{}  {}", icon, label);
                Self::draw_text_str(
                    target,
                    &display_text,
                    &dwrite.format_regular,
                    &D2D_RECT_F {
                        left: tab_rect.left + 14.0,
                        top: tab_rect.top,
                        right: tab_rect.right - 8.0,
                        bottom: tab_rect.bottom,
                    },
                    brush,
                );

                regions.push(ClickRegion {
                    rect: tab_rect,
                    action: SettingsAction::SelectTab(idx),
                });
            }

            // Bottom sidebar footer
            brush.SetColor(&COLOR_TEXT_MUTED);
            Self::draw_text_str(
                target,
                "✓ Otomatis tersimpan",
                &dwrite.format_small,
                &D2D_RECT_F {
                    left: 18.0,
                    top: height - 34.0,
                    right: sidebar_w - 18.0,
                    bottom: height - 14.0,
                },
                brush,
            );

            // ================= RIGHT CONTENT AREA =================
            let content_x = sidebar_w + 24.0;
            let content_w = width - content_x - 24.0;
            let mut cur_y = 24.0;

            let (header_title, header_desc) = match active_tab {
                0 => ("Pengaturan Umum", "Konfigurasi dasar perilaku startup dan interaksi notch."),
                1 => ("Kustomisasi Seksi Notch", "Aktifkan atau nonaktifkan bagian notch sesuai kebutuhan alur kerja Anda."),
                2 => ("AI Usage Monitor", "Pantau kuota penggunaan model AI favorit Anda secara langsung di notch."),
                3 => ("Musik & Lirik", "Pendaran warna dinamis, lirik real-time, dan status Rich Presence Discord."),
                4 => ("Produktivitas", "Pengatur waktu fokus kerja Pomodoro dan pengingat hidrasi tubuh."),
                5 => ("Tampilan & Desain", "Sesuaikan warna aksen dan transparansi visual acrylic."),
                _ => ("Tentang Nocan", "Informasi aplikasi, sumber daya pihak ketiga, dan reset konfigurasi."),
            };

            // Header Title & Subtitle
            brush.SetColor(&COLOR_TEXT_PRIMARY);
            Self::draw_text_str(
                target,
                header_title,
                &dwrite.format_title,
                &D2D_RECT_F {
                    left: content_x,
                    top: cur_y,
                    right: content_x + content_w,
                    bottom: cur_y + 26.0,
                },
                brush,
            );
            cur_y += 24.0;

            brush.SetColor(&COLOR_TEXT_SECONDARY);
            Self::draw_text_str(
                target,
                header_desc,
                &dwrite.format_sub,
                &D2D_RECT_F {
                    left: content_x,
                    top: cur_y,
                    right: content_x + content_w,
                    bottom: cur_y + 20.0,
                },
                brush,
            );
            cur_y += 30.0;

            // Render cards for active tab
            let card_h = 52.0f32;
            let card_spacing = 8.0f32;

            match active_tab {
                0 => {
                    // TAB 0: GENERAL
                    let rows = [
                        (
                            "Mulai Otomatis saat Windows Login",
                            "Jalankan Nocan Living Notch di latar belakang saat PC menyala.",
                            settings.general.autostart,
                            SettingsAction::ToggleAutostart,
                        ),
                        (
                            "Maskot Kucing Nocan",
                            "Tampilkan animasi maskot kucing interaktif di sisi kiri notch.",
                            settings.general.mascot_enabled,
                            SettingsAction::ToggleMascot,
                        ),
                        (
                            "Mode Minimal Notch",
                            "Tampilan ringkas berbentuk kapsul kecil saat tidak ada musik/aktivitas.",
                            settings.general.minimal_notch,
                            SettingsAction::ToggleMinimalNotch,
                        ),
                        (
                            "Auto-Minimize saat Window Penuh",
                            "Otomatis ciutkan notch saat ada aplikasi jendela yang dimaksimalkan.",
                            settings.general.auto_minimize_on_maximize,
                            SettingsAction::ToggleAutoMinimizeOnMaximize,
                        ),
                    ];

                    for (title, desc, active, action) in rows {
                        let card_rect = D2D_RECT_F {
                            left: content_x,
                            top: cur_y,
                            right: content_x + content_w,
                            bottom: cur_y + card_h,
                        };

                        Self::render_toggle_card(
                            target,
                            brush,
                            dwrite,
                            &card_rect,
                            title,
                            desc,
                            active,
                            mouse_pos,
                        );

                        regions.push(ClickRegion {
                            rect: card_rect,
                            action,
                        });

                        cur_y += card_h + card_spacing;
                    }
                }
                1 => {
                    // TAB 1: SEKSI NOTCH (SECTION TOGGLES)
                    let sec_card_h = 44.0f32;
                    let sec_card_gap = 6.0f32;
                    let sec_rows = [
                        (
                            "🎵  Seksi Musik & Lirik",
                            "Pemutar media, kontrol lagu, dan lirik tersinkronisasi.",
                            settings.sections.show_music,
                            SettingsAction::ToggleSectionMusic,
                        ),
                        (
                            "⏱️  Seksi Fokus Pomodoro",
                            "Timer fokus kerja 25 menit dan interval istirahat produktif.",
                            settings.sections.show_focus,
                            SettingsAction::ToggleSectionFocus,
                        ),
                        (
                            "💧  Seksi Pengingat Air",
                            "Pelacak target hidrasi harian dan pencatatan asupan air.",
                            settings.sections.show_water,
                            SettingsAction::ToggleSectionWater,
                        ),
                        (
                            "📁  Seksi File Shelf",
                            "Wadah penyimpanan cepat drag-and-drop berkas sementara.",
                            settings.sections.show_shelf,
                            SettingsAction::ToggleSectionShelf,
                        ),
                        (
                            "📝  Seksi To-Do List",
                            "Daftar checklist tugas harian interaktif di notch.",
                            settings.sections.show_todo,
                            SettingsAction::ToggleSectionTodo,
                        ),
                        (
                            "🤖  Seksi OpenMaus AI Bot",
                            "Asisten AI percakapan cepat dan pintasan prompt cerdas.",
                            settings.sections.show_openmaus,
                            SettingsAction::ToggleSectionOpenMaus,
                        ),
                        (
                            "📊  Seksi AI Usage Monitor",
                            "Pantau kuota token Antigravity, Claude, Codex, dan OpenCode.",
                            settings.sections.show_ai_usage,
                            SettingsAction::ToggleSectionAiUsage,
                        ),
                        (
                            "📈  Seksi Saham & Bursa",
                            "Pantau harga saham IHSG (BBCA, BBRI, TLKM) dan global (NVDA, AAPL, BTC).",
                            settings.sections.show_saham,
                            SettingsAction::ToggleSectionSaham,
                        ),
                    ];

                    for (title, desc, active, action) in sec_rows {
                        let card_rect = D2D_RECT_F {
                            left: content_x,
                            top: cur_y,
                            right: content_x + content_w,
                            bottom: cur_y + sec_card_h,
                        };

                        Self::render_toggle_card(
                            target,
                            brush,
                            dwrite,
                            &card_rect,
                            title,
                            desc,
                            active,
                            mouse_pos,
                        );

                        regions.push(ClickRegion {
                            rect: card_rect,
                            action,
                        });

                        cur_y += sec_card_h + sec_card_gap;
                    }
                }
                2 => {
                    // TAB 2: AI MONITOR
                    let provider_rows = [
                        (
                            "Google Antigravity",
                            "Pantau kuota request, rate limit, dan token Antigravity / Gemini.",
                            settings.ai.show_antigravity,
                            SettingsAction::ToggleAiAntigravity,
                        ),
                        (
                            "Anthropic Claude",
                            "Pantau kuota token dan status pemakaian API Claude.",
                            settings.ai.show_claude,
                            SettingsAction::ToggleAiClaude,
                        ),
                        (
                            "OpenAI Codex",
                            "Pantau kuota token dan sisa batas rate limit OpenAI Codex.",
                            settings.ai.show_codex,
                            SettingsAction::ToggleAiCodex,
                        ),
                        (
                            "OpenCode Assistant",
                            "Pantau kuota token dan status OpenCode local/remote engine.",
                            settings.ai.show_opencode,
                            SettingsAction::ToggleAiOpenCode,
                        ),
                        (
                            "Bilah Proporsi Penggunaan",
                            "Tampilkan visual bar perbandingan kuota provider di expanded notch.",
                            settings.ai.show_proportional_bar,
                            SettingsAction::ToggleAiProportionBar,
                        ),
                    ];

                    for (title, desc, active, action) in provider_rows {
                        let card_rect = D2D_RECT_F {
                            left: content_x,
                            top: cur_y,
                            right: content_x + content_w,
                            bottom: cur_y + card_h,
                        };

                        Self::render_toggle_card(
                            target,
                            brush,
                            dwrite,
                            &card_rect,
                            title,
                            desc,
                            active,
                            mouse_pos,
                        );

                        regions.push(ClickRegion {
                            rect: card_rect,
                            action,
                        });

                        cur_y += card_h + card_spacing;
                    }

                    // Polling interval stepper card
                    let poll_rect = D2D_RECT_F {
                        left: content_x,
                        top: cur_y,
                        right: content_x + content_w,
                        bottom: cur_y + card_h,
                    };
                    Self::render_base_card(target, brush, dwrite, &poll_rect, "Interval Polling AI", "Frekuensi cek pembaruan data kuota AI ke API lokal/remote.", mouse_pos);
                    Self::draw_stepper(
                        target,
                        brush,
                        dwrite,
                        poll_rect.right - 120.0,
                        poll_rect.top + (card_h - 26.0) * 0.5,
                        &format!("{} dtk", settings.ai.poll_interval_secs),
                        SettingsAction::AdjustAiPoll(-1),
                        SettingsAction::AdjustAiPoll(1),
                        &mut regions,
                    );
                }
                3 => {
                    // TAB 3: MUSIC & LYRICS
                    let music_rows = [
                        (
                            "Dynamic Ambience Glow",
                            "Pendaran warna halus mengikuti dominan warna cover album yang diputar.",
                            settings.music.ambient_glow_enabled,
                            SettingsAction::ToggleMusicGlow,
                        ),
                        (
                            "Lebar Lirik Dinamis",
                            "Sesuaikan lebar notch otomatis secara mulus dengan panjang lirik.",
                            settings.music.dynamic_lyrics_width,
                            SettingsAction::ToggleDynamicLyricsWidth,
                        ),
                        (
                            "Discord Rich Presence (RPC)",
                            "Tampilkan lagu yang sedang Anda dengarkan di status profil Discord.",
                            settings.music.discord_rpc_enabled,
                            SettingsAction::ToggleDiscordRpc,
                        ),
                    ];

                    for (title, desc, active, action) in music_rows {
                        let card_rect = D2D_RECT_F {
                            left: content_x,
                            top: cur_y,
                            right: content_x + content_w,
                            bottom: cur_y + card_h,
                        };

                        Self::render_toggle_card(
                            target,
                            brush,
                            dwrite,
                            &card_rect,
                            title,
                            desc,
                            active,
                            mouse_pos,
                        );

                        regions.push(ClickRegion {
                            rect: card_rect,
                            action,
                        });

                        cur_y += card_h + card_spacing;
                    }
                }
                4 => {
                    // TAB 4: PRODUCTIVITY (POMODORO & WATER)
                    let p_cards = [
                        (
                            "Durasi Kerja Pomodoro",
                            "Lama waktu per sesi fokus kerja.",
                            format!("{} mnt", settings.productivity.focus_work_mins),
                            SettingsAction::AdjustFocusWorkMins(-5),
                            SettingsAction::AdjustFocusWorkMins(5),
                        ),
                        (
                            "Durasi Jeda Istirahat",
                            "Lama waktu per sesi rehat pendek.",
                            format!("{} mnt", settings.productivity.focus_break_mins),
                            SettingsAction::AdjustFocusBreakMins(-1),
                            SettingsAction::AdjustFocusBreakMins(1),
                        ),
                        (
                            "Interval Pengingat Minum Air",
                            "Frekuensi kemunculan notifikasi minum air.",
                            format!("{} mnt", settings.productivity.water_interval_mins),
                            SettingsAction::AdjustWaterIntervalMins(-15),
                            SettingsAction::AdjustWaterIntervalMins(15),
                        ),
                        (
                            "Target Minum Harian",
                            "Jumlah gelas air yang ingin dicapai dalam satu hari.",
                            format!("{} gls", settings.productivity.water_daily_target_glasses),
                            SettingsAction::AdjustWaterTargetGlasses(-1),
                            SettingsAction::AdjustWaterTargetGlasses(1),
                        ),
                    ];

                    for (title, desc, val_str, dec, inc) in p_cards {
                        let card_rect = D2D_RECT_F {
                            left: content_x,
                            top: cur_y,
                            right: content_x + content_w,
                            bottom: cur_y + card_h,
                        };

                        Self::render_base_card(target, brush, dwrite, &card_rect, title, desc, mouse_pos);
                        Self::draw_stepper(
                            target,
                            brush,
                            dwrite,
                            card_rect.right - 120.0,
                            card_rect.top + (card_h - 26.0) * 0.5,
                            &val_str,
                            dec,
                            inc,
                            &mut regions,
                        );

                        cur_y += card_h + card_spacing;
                    }
                }
                5 => {
                    // TAB 5: APPEARANCE
                    let colors = [
                        ("Auto", D2D1_COLOR_F { r: 0.35, g: 0.35, b: 0.40, a: 1.0 }),
                        ("Gemini", D2D1_COLOR_F { r: 0.23, g: 0.51, b: 0.96, a: 1.0 }),
                        ("Coral", D2D1_COLOR_F { r: 0.95, g: 0.45, b: 0.30, a: 1.0 }),
                        ("Emerald", D2D1_COLOR_F { r: 0.20, g: 0.75, b: 0.50, a: 1.0 }),
                        ("Cyan", D2D1_COLOR_F { r: 0.15, g: 0.70, b: 0.85, a: 1.0 }),
                        ("Violet", D2D1_COLOR_F { r: 0.65, g: 0.35, b: 0.95, a: 1.0 }),
                    ];

                    let accent_rect = D2D_RECT_F {
                        left: content_x,
                        top: cur_y,
                        right: content_x + content_w,
                        bottom: cur_y + 64.0,
                    };
                    Self::render_base_card(target, brush, dwrite, &accent_rect, "Warna Aksen Aplikasi", "Pilih palet tema aksen notch dan indikator.", mouse_pos);

                    // Draw color pills
                    let pill_w = 48.0f32;
                    let pill_h = 24.0f32;
                    let mut pill_x = accent_rect.right - (pill_w + 6.0) * 6.0 - 4.0;
                    let pill_y = accent_rect.top + (64.0 - pill_h) * 0.5;

                    for (m_idx, (m_name, m_color)) in colors.iter().enumerate() {
                        let pill_rect = D2D_RECT_F {
                            left: pill_x,
                            top: pill_y,
                            right: pill_x + pill_w,
                            bottom: pill_y + pill_h,
                        };

                        let is_sel = settings.appearance.accent_mode == m_idx as u32;
                        brush.SetColor(m_color);
                        target.FillRoundedRectangle(
                            &D2D1_ROUNDED_RECT {
                                rect: pill_rect,
                                radiusX: 12.0,
                                radiusY: 12.0,
                            },
                            brush,
                        );

                        if is_sel {
                            brush.SetColor(&COLOR_TEXT_PRIMARY);
                            target.DrawRoundedRectangle(
                                &D2D1_ROUNDED_RECT {
                                    rect: pill_rect,
                                    radiusX: 12.0,
                                    radiusY: 12.0,
                                },
                                brush,
                                2.0,
                                None,
                            );
                        }

                        brush.SetColor(&COLOR_TEXT_PRIMARY);
                        Self::draw_text_str(target, m_name, &dwrite.format_small_center, &pill_rect, brush);

                        regions.push(ClickRegion {
                            rect: pill_rect,
                            action: SettingsAction::SetAccentMode(m_idx as u32),
                        });

                        pill_x += pill_w + 6.0;
                    }

                    cur_y += 64.0 + card_spacing;

                    // Opacity Stepper
                    let op_rect = D2D_RECT_F {
                        left: content_x,
                        top: cur_y,
                        right: content_x + content_w,
                        bottom: cur_y + card_h,
                    };
                    Self::render_base_card(target, brush, dwrite, &op_rect, "Opasitas Latar Acrylic", "Tingkat transparansi notch terhadap wallpaper desktop.", mouse_pos);
                    Self::draw_stepper(
                        target,
                        brush,
                        dwrite,
                        op_rect.right - 120.0,
                        op_rect.top + (card_h - 26.0) * 0.5,
                        &format!("{}%", (settings.appearance.acrylic_opacity * 100.0).round() as i32),
                        SettingsAction::AdjustAcrylicOpacity(-0.05),
                        SettingsAction::AdjustAcrylicOpacity(0.05),
                        &mut regions,
                    );
                }
                _ => {
                    // TAB 5: ABOUT
                    let info_rect = D2D_RECT_F {
                        left: content_x,
                        top: cur_y,
                        right: content_x + content_w,
                        bottom: cur_y + 110.0,
                    };
                    brush.SetColor(&COLOR_CARD_BG);
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: info_rect,
                            radiusX: 8.0,
                            radiusY: 8.0,
                        },
                        brush,
                    );
                    brush.SetColor(&COLOR_CARD_BORDER);
                    target.DrawRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: info_rect,
                            radiusX: 8.0,
                            radiusY: 8.0,
                        },
                        brush,
                        1.0,
                        None,
                    );

                    brush.SetColor(&COLOR_TEXT_PRIMARY);
                    Self::draw_text_str(
                        target,
                        "Nocan Living Notch v1.1.0 (Direct2D Native)",
                        &dwrite.format_regular_bold,
                        &D2D_RECT_F {
                            left: info_rect.left + 16.0,
                            top: info_rect.top + 14.0,
                            right: info_rect.right - 16.0,
                            bottom: info_rect.top + 34.0,
                        },
                        brush,
                    );

                    brush.SetColor(&COLOR_TEXT_SECONDARY);
                    let info_text = "Notch modern dan cerdas untuk Windows 11 dengan pelacak musik, sinkronisasi lirik LRCLIB, Pomodoro, pengingat minum air, file shelf, to-do list, dan AI Usage Monitor.\n\nDikembangkan secara performan dengan Rust murni dan GPU DirectX/DirectComposition.";
                    Self::draw_text_str(
                        target,
                        info_text,
                        &dwrite.format_sub,
                        &D2D_RECT_F {
                            left: info_rect.left + 16.0,
                            top: info_rect.top + 38.0,
                            right: info_rect.right - 16.0,
                            bottom: info_rect.bottom - 10.0,
                        },
                        brush,
                    );

                    cur_y += 110.0 + 16.0;

                    // Reset button card
                    let reset_card_rect = D2D_RECT_F {
                        left: content_x,
                        top: cur_y,
                        right: content_x + content_w,
                        bottom: cur_y + card_h,
                    };
                    Self::render_base_card(
                        target,
                        brush,
                        dwrite,
                        &reset_card_rect,
                        "Reset Seluruh Pengaturan",
                        "Kembalikan seluruh preferensi ke konfigurasi bawaan.",
                        mouse_pos,
                    );

                    // Reset button
                    let btn_rect = D2D_RECT_F {
                        left: reset_card_rect.right - 130.0,
                        top: reset_card_rect.top + (card_h - 28.0) * 0.5,
                        right: reset_card_rect.right - 16.0,
                        bottom: reset_card_rect.top + (card_h + 28.0) * 0.5,
                    };

                    brush.SetColor(&COLOR_DANGER);
                    target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: btn_rect,
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        brush,
                    );
                    brush.SetColor(&COLOR_TEXT_PRIMARY);
                    Self::draw_text_str(target, "Reset Default", &dwrite.format_button_center, &btn_rect, brush);

                    regions.push(ClickRegion {
                        rect: btn_rect,
                        action: SettingsAction::ResetToDefaults,
                    });
                }
            }
        }

        regions
    }

    fn render_base_card(
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        dwrite: &DWriteContext,
        rect: &D2D_RECT_F,
        title: &str,
        desc: &str,
        mouse_pos: (f32, f32),
    ) {
        let is_hover = mouse_pos.0 >= rect.left
            && mouse_pos.0 <= rect.right
            && mouse_pos.1 >= rect.top
            && mouse_pos.1 <= rect.bottom;

        unsafe {
            brush.SetColor(if is_hover { &COLOR_CARD_HOVER } else { &COLOR_CARD_BG });
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: *rect,
                    radiusX: 8.0,
                    radiusY: 8.0,
                },
                brush,
            );

            brush.SetColor(&COLOR_CARD_BORDER);
            target.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: *rect,
                    radiusX: 8.0,
                    radiusY: 8.0,
                },
                brush,
                1.0,
                None,
            );

            // Title
            brush.SetColor(&COLOR_TEXT_PRIMARY);
            Self::draw_text_str(
                target,
                title,
                &dwrite.format_regular_bold,
                &D2D_RECT_F {
                    left: rect.left + 14.0,
                    top: rect.top + 8.0,
                    right: rect.right - 140.0,
                    bottom: rect.top + 26.0,
                },
                brush,
            );

            // Subtitle
            brush.SetColor(&COLOR_TEXT_SECONDARY);
            Self::draw_text_str(
                target,
                desc,
                &dwrite.format_sub,
                &D2D_RECT_F {
                    left: rect.left + 14.0,
                    top: rect.top + 28.0,
                    right: rect.right - 140.0,
                    bottom: rect.bottom - 6.0,
                },
                brush,
            );
        }
    }

    fn render_toggle_card(
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        dwrite: &DWriteContext,
        rect: &D2D_RECT_F,
        title: &str,
        desc: &str,
        active: bool,
        mouse_pos: (f32, f32),
    ) {
        Self::render_base_card(target, brush, dwrite, rect, title, desc, mouse_pos);
        Self::draw_toggle(
            target,
            brush,
            rect.right - 58.0,
            rect.top + (rect.bottom - rect.top - 22.0) * 0.5,
            active,
        );
    }
}
