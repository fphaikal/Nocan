use windows::core::{Result, PCWSTR};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_CLIP,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteTextFormat,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_LIGHT, DWRITE_FONT_WEIGHT_MEDIUM,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
    DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TEXT_METRICS, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};

pub struct DWriteContext {
    pub factory: IDWriteFactory,
    pub font_name: String,
    pub format_title: IDWriteTextFormat,        // 14.5px SemiBold, left, v-center
    pub format_regular: IDWriteTextFormat,      // 12.5px Normal, left, v-center
    pub format_regular_bold: IDWriteTextFormat, // 12.5px SemiBold, left, v-center
    pub format_regular_right: IDWriteTextFormat,// 12.5px Normal, right, v-center
    pub format_regular_semibold_right: IDWriteTextFormat, // 12.5px SemiBold, right, v-center
    pub format_peek_pct: IDWriteTextFormat,     // 23px SemiBold, right, v-center
    pub format_sub: IDWriteTextFormat,          // 11px Normal, left, v-center
    pub format_sub_center: IDWriteTextFormat,   // 11px Normal, center, v-center
    pub format_small: IDWriteTextFormat,        // 9.5px Medium, left, v-center
    pub format_small_center: IDWriteTextFormat, // 9.5px Medium, center, v-center
    pub format_card_name: IDWriteTextFormat,    // 10.5px SemiBold, center, v-center
    pub format_big: IDWriteTextFormat,          // 30px Light, left, v-center
    pub format_big_center: IDWriteTextFormat,   // 31px Light, center, v-center
    pub format_idle_time: IDWriteTextFormat,    // 16px SemiBold, center, v-center
    pub format_center: IDWriteTextFormat,       // 12.5px Normal, center, v-center
    pub format_button: IDWriteTextFormat,       // 11.5px Normal, left, v-center
    pub format_button_center: IDWriteTextFormat,// 11.5px SemiBold, center, v-center
    pub format_lyric: IDWriteTextFormat,        // 15px SemiBold, left, v-center
    pub format_lyric_bar: IDWriteTextFormat,    // 13px SemiBold, center, v-center
    pub format_timer: IDWriteTextFormat,        // 39px Light, center, v-center
    pub format_page_title: IDWriteTextFormat,        // 23px SemiBold, left, v-center
    pub format_page_sub: IDWriteTextFormat,          // 12.5px Normal, left, v-center
    pub format_section_header: IDWriteTextFormat,    // 13.5px SemiBold, left, v-center
    pub format_section_desc: IDWriteTextFormat,      // 12.0px Normal, left, v-center
    pub format_item_title: IDWriteTextFormat,        // 13.0px Medium, left, v-center
    pub format_item_desc: IDWriteTextFormat,         // 12.0px Normal, left, v-center
    pub format_sidebar_header: IDWriteTextFormat,    // 11.0px Normal, left, v-center
    pub format_sidebar_item: IDWriteTextFormat,      // 13.0px Normal, left, v-center
    pub format_sidebar_active: IDWriteTextFormat,    // 13.0px SemiBold, left, v-center
    pub format_control_text: IDWriteTextFormat,      // 12.0px Medium, center, v-center
    pub format_badge: IDWriteTextFormat,             // 10.5px Medium, center, v-center
    pub format_code_pill: IDWriteTextFormat,         // 11.0px Normal, center, v-center
    pub format_profile_name: IDWriteTextFormat,      // 12.5px SemiBold, left, v-center
    pub format_profile_email: IDWriteTextFormat,     // 11.0px Normal, left, v-center
    pub format_close_x: IDWriteTextFormat,           // 13.0px Normal, center, v-center
}

pub fn resolve_font_family(factory: &IDWriteFactory, preferred: Option<&str>) -> String {
    let mut collection: Option<IDWriteFontCollection> = None;
    let _ = unsafe { factory.GetSystemFontCollection(&mut collection, false) };

    let check_exists = |name: &str| -> bool {
        if let Some(ref col) = collection {
            let utf16: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let mut index = 0u32;
            let mut exists = windows::Win32::Foundation::BOOL(0);
            unsafe {
                if col.FindFamilyName(PCWSTR(utf16.as_ptr()), &mut index, &mut exists).is_ok() {
                    return exists.as_bool();
                }
            }
        }
        false
    };

    if let Some(pref) = preferred {
        if !pref.is_empty() && check_exists(pref) {
            return pref.to_string();
        }
    }

    let candidates = [
        "Inter",
        "SF Pro Text",
        "SF Pro",
        "SF Pro Display",
        "Segoe UI Variable Text",
        "Segoe UI Variable Display",
        "Segoe UI",
    ];

    for candidate in candidates {
        if check_exists(candidate) {
            return candidate.to_string();
        }
    }

    "Segoe UI".to_string()
}

impl DWriteContext {
    pub fn new() -> Result<Self> {
        let settings = crate::config::AppSettings::load();
        let pref = if settings.appearance.font_family.is_empty() {
            None
        } else {
            Some(settings.appearance.font_family.as_str())
        };
        Self::with_font_name(pref)
    }

    pub fn with_font_name(preferred: Option<&str>) -> Result<Self> {
        let factory: IDWriteFactory = unsafe {
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?
        };

        let resolved_font = resolve_font_family(&factory, preferred);
        let font_family_utf16: Vec<u16> = resolved_font.encode_utf16().chain(std::iter::once(0)).collect();
        let locale: Vec<u16> = "en-US\0".encode_utf16().collect();

        let make = |size: f32, weight: DWRITE_FONT_WEIGHT, align: i32| -> Result<IDWriteTextFormat> {
            unsafe {
                let fmt = factory.CreateTextFormat(
                    PCWSTR(font_family_utf16.as_ptr()),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    PCWSTR(locale.as_ptr()),
                )?;
                let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                let _ = fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                if let Ok(ellipsis) = factory.CreateEllipsisTrimmingSign(&fmt) {
                    let trimming = DWRITE_TRIMMING {
                        granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                        delimiter: 0,
                        delimiterCount: 0,
                    };
                    let _ = fmt.SetTrimming(&trimming, &ellipsis);
                }
                match align {
                    1 => {
                        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                    }
                    2 => {
                        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING);
                    }
                    _ => {
                        let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                    }
                }
                Ok(fmt)
            }
        };

        Ok(Self {
            font_name: resolved_font,
            format_title: make(14.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_regular: make(12.5, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_regular_bold: make(12.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_regular_right: make(12.5, DWRITE_FONT_WEIGHT_NORMAL, 2)?,
            format_regular_semibold_right: make(12.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 2)?,
            format_peek_pct: make(23.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 2)?,
            format_sub: make(11.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_sub_center: make(11.0, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_small: make(9.5, DWRITE_FONT_WEIGHT_MEDIUM, 0)?,
            format_small_center: make(9.5, DWRITE_FONT_WEIGHT_MEDIUM, 1)?,
            format_card_name: make(10.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_big: make(30.0, DWRITE_FONT_WEIGHT_LIGHT, 0)?,
            format_big_center: make(31.0, DWRITE_FONT_WEIGHT_LIGHT, 1)?,
            format_idle_time: make(16.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_center: make(12.5, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_button: make(11.5, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_button_center: make(11.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_lyric: make(15.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_lyric_bar: make(13.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_timer: make(39.0, DWRITE_FONT_WEIGHT_LIGHT, 1)?,
            format_page_title: make(20.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_page_sub: make(12.5, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_section_header: make(13.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_section_desc: make(12.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_item_title: make(13.0, DWRITE_FONT_WEIGHT_MEDIUM, 0)?,
            format_item_desc: make(12.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_sidebar_header: make(11.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_sidebar_item: make(14.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_sidebar_active: make(14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_control_text: make(13.0, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_badge: make(10.5, DWRITE_FONT_WEIGHT_MEDIUM, 1)?,
            format_code_pill: make(11.5, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_profile_name: make(12.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_profile_email: make(11.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_close_x: make(13.0, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            factory,
        })
    }

    pub fn measure_text_width(&self, text: &str, format: &IDWriteTextFormat) -> f32 {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            if let Ok(layout) = self.factory.CreateTextLayout(&utf16, format, 4000.0, 100.0) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    return m.widthIncludingTrailingWhitespace.max(m.width);
                }
            }
        }
        0.0
    }

    pub fn draw_text(
        &self,
        context: &ID2D1DeviceContext,
        text: &str,
        rect: D2D_RECT_F,
        brush: &ID2D1SolidColorBrush,
        format: &IDWriteTextFormat,
    ) {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            context.DrawText(
                &utf16,
                format,
                &rect,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_resolution() {
        let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap() };
        let default_font = resolve_font_family(&factory, None);
        let mut collection = None;
        unsafe { factory.GetSystemFontCollection(&mut collection, false).unwrap() };
        let collection = collection.unwrap();
        let candidates = ["Inter", "SF Pro Text", "SF Pro", "SF Pro Display",
            "Segoe UI Variable Text", "Segoe UI Variable Display", "Segoe UI"];
        let expected = candidates.into_iter().find(|name| {
            let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let mut index = 0;
            let mut exists = windows::Win32::Foundation::BOOL(0);
            unsafe { collection.FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists).unwrap() };
            exists.as_bool()
        }).unwrap_or("Segoe UI");
        assert_eq!(default_font, expected);
        assert_eq!(resolve_font_family(&factory, Some("")), default_font);
        assert_eq!(resolve_font_family(&factory, Some("Nocan Missing Font")), default_font);
        assert_eq!(resolve_font_family(&factory, Some("SF Pro Text")), default_font);

        let segoe = resolve_font_family(&factory, Some("Segoe UI"));
        assert_eq!(segoe, "Segoe UI");
    }
}
