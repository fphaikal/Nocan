use windows::core::{Result, PCWSTR};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_CLIP,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};

pub struct DWriteContext {
    pub factory: IDWriteFactory,
    pub format_title: IDWriteTextFormat,        // 15px SemiBold, left, v-center
    pub format_regular: IDWriteTextFormat,      // 13px Normal, left, v-center
    pub format_regular_bold: IDWriteTextFormat, // 13px Bold, left, v-center
    pub format_regular_right: IDWriteTextFormat,// 13px Normal, right, v-center
    pub format_sub: IDWriteTextFormat,          // 11.5px Normal, left, v-center
    pub format_sub_center: IDWriteTextFormat,   // 11.5px Normal, center, v-center
    pub format_small: IDWriteTextFormat,        // 9.5px Normal, left, v-center
    pub format_small_center: IDWriteTextFormat, // 9.5px Normal, center, v-center
    pub format_card_name: IDWriteTextFormat,    // 10.5px SemiBold, center, v-center
    pub format_big: IDWriteTextFormat,          // 30px Bold, left, v-center
    pub format_big_center: IDWriteTextFormat,   // 31px Bold, center, v-center
    pub format_idle_time: IDWriteTextFormat,    // 16px Bold, center, v-center
    pub format_center: IDWriteTextFormat,       // 13px Normal, center, v-center
    pub format_button: IDWriteTextFormat,       // 12px Normal, left, v-center
    pub format_button_center: IDWriteTextFormat,// 12px SemiBold, center, v-center
    pub format_lyric: IDWriteTextFormat,        // 15.5px Bold, left, v-center
    pub format_nav: IDWriteTextFormat,          // 18px SemiBold, center, v-center
}

impl DWriteContext {
    pub fn new() -> Result<Self> {
        let factory: IDWriteFactory = unsafe {
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?
        };

        let make = |size: f32, weight: DWRITE_FONT_WEIGHT, align: i32| -> Result<IDWriteTextFormat> {
            let font_family: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let locale: Vec<u16> = "en-US\0".encode_utf16().collect();
            unsafe {
                let fmt = factory.CreateTextFormat(
                    PCWSTR(font_family.as_ptr()),
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
            format_title: make(15.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 0)?,
            format_regular: make(13.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_regular_bold: make(13.0, DWRITE_FONT_WEIGHT_BOLD, 0)?,
            format_regular_right: make(13.0, DWRITE_FONT_WEIGHT_NORMAL, 2)?,
            format_sub: make(11.5, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_sub_center: make(11.5, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_small: make(9.5, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_small_center: make(9.5, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_card_name: make(10.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_big: make(30.0, DWRITE_FONT_WEIGHT_BOLD, 0)?,
            format_big_center: make(31.0, DWRITE_FONT_WEIGHT_BOLD, 1)?,
            format_idle_time: make(16.0, DWRITE_FONT_WEIGHT_BOLD, 1)?,
            format_center: make(13.0, DWRITE_FONT_WEIGHT_NORMAL, 1)?,
            format_button: make(12.0, DWRITE_FONT_WEIGHT_NORMAL, 0)?,
            format_button_center: make(12.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            format_lyric: make(15.5, DWRITE_FONT_WEIGHT_BOLD, 0)?,
            format_nav: make(18.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 1)?,
            factory,
        })
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
