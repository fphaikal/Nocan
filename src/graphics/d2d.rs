use windows::core::Result;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
    ID2D1SolidColorBrush, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGISurface};

pub struct D2DContext {
    pub factory: ID2D1Factory1,
    pub device: ID2D1Device,
    pub context: ID2D1DeviceContext,
    target_bitmap: Option<ID2D1Bitmap1>,
}

impl D2DContext {
    pub fn new(dxgi_device: &IDXGIDevice) -> Result<Self> {
        let factory_options = D2D1_FACTORY_OPTIONS::default();
        let factory: ID2D1Factory1 = unsafe {
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&factory_options))?
        };

        let device: ID2D1Device = unsafe { factory.CreateDevice(dxgi_device)? };
        let context: ID2D1DeviceContext = unsafe {
            device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?
        };

        Ok(Self {
            factory,
            device,
            context,
            target_bitmap: None,
        })
    }

    pub fn set_target_surface(&mut self, surface: &IDXGISurface) -> Result<()> {
        let pixel_format = D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_PREMULTIPLIED,
        };

        let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: pixel_format,
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            colorContext: std::mem::ManuallyDrop::new(None),
        };

        let bitmap: ID2D1Bitmap1 = unsafe {
            self.context.CreateBitmapFromDxgiSurface(surface, Some(&bitmap_props))?
        };

        unsafe {
            self.context.SetTarget(&bitmap);
        }

        self.target_bitmap = Some(bitmap);
        Ok(())
    }

    pub fn reset_target(&mut self) {
        unsafe {
            self.context.SetTarget(None);
        }
        self.target_bitmap = None;
    }

    pub fn begin_draw(&self) {
        unsafe {
            self.context.BeginDraw();
        }
    }

    pub fn end_draw(&self) -> Result<()> {
        unsafe {
            self.context.EndDraw(None, None)?;
        }
        Ok(())
    }

    pub fn clear(&self, color: D2D1_COLOR_F) {
        unsafe {
            self.context.Clear(Some(&color as *const _));
        }
    }

    pub fn create_solid_brush(&self, color: D2D1_COLOR_F) -> Result<ID2D1SolidColorBrush> {
        unsafe { self.context.CreateSolidColorBrush(&color as *const _, None) }
    }

    pub fn fill_rounded_rect(&self, rect: D2D_RECT_F, rx: f32, ry: f32, brush: &ID2D1SolidColorBrush) {
        let rounded = D2D1_ROUNDED_RECT {
            rect,
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.FillRoundedRectangle(&rounded, brush);
        }
    }

    pub fn draw_rounded_rect(
        &self,
        rect: D2D_RECT_F,
        rx: f32,
        ry: f32,
        brush: &ID2D1SolidColorBrush,
        stroke_width: f32,
    ) {
        let rounded = D2D1_ROUNDED_RECT {
            rect,
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.DrawRoundedRectangle(&rounded, brush, stroke_width, None);
        }
    }

    pub fn fill_rect(&self, rect: D2D_RECT_F, brush: &ID2D1SolidColorBrush) {
        unsafe {
            self.context.FillRectangle(&rect, brush);
        }
    }

    pub fn fill_ellipse(&self, center_x: f32, center_y: f32, rx: f32, ry: f32, brush: &ID2D1SolidColorBrush) {
        let ellipse = windows::Win32::Graphics::Direct2D::D2D1_ELLIPSE {
            point: D2D_POINT_2F { x: center_x, y: center_y },
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.FillEllipse(&ellipse, brush);
        }
    }
}
