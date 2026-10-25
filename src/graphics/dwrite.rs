use windows::core::{Result, PCWSTR};
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext;
use windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush;
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER,
};

pub struct DWriteContext {
    pub factory: IDWriteFactory,
    pub format_title: IDWriteTextFormat,
    pub format_regular: IDWriteTextFormat,
    pub format_small: IDWriteTextFormat,
    pub format_big: IDWriteTextFormat,
    pub format_center: IDWriteTextFormat,
    pub format_button: IDWriteTextFormat,
    pub format_nav: IDWriteTextFormat,
}

impl DWriteContext {
    pub fn new() -> Result<Self> {
        let factory: IDWriteFactory = unsafe {
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?
        };

        let font_family: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
        let font_name = PCWSTR(font_family.as_ptr());
        let locale: Vec<u16> = "en-US\0".encode_utf16().collect();
        let locale_name = PCWSTR(locale.as_ptr());

        let format_title = unsafe {
            factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                11.5,
                locale_name,
            )?
        };

        let format_regular = unsafe {
            factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                9.5,
                locale_name,
            )?
        };

        let format_small = unsafe {
            factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                8.5,
                locale_name,
            )?
        };

        let format_big = unsafe {
            factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                20.0,
                locale_name,
            )?
        };

        let format_center = unsafe {
            let fmt = factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                9.0,
                locale_name,
            )?;
            let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            fmt
        };

        let format_button = unsafe {
            let fmt = factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                9.5,
                locale_name,
            )?;
            let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            fmt
        };

        let format_nav = unsafe {
            let fmt = factory.CreateTextFormat(
                font_name,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                18.0,
                locale_name,
            )?;
            let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            fmt
        };

        Ok(Self {
            factory,
            format_title,
            format_regular,
            format_small,
            format_big,
            format_center,
            format_button,
            format_nav,
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
                windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_CLIP,
                windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }
}
