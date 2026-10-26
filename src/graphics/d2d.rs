use windows::core::Result;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN, D2D1_GRADIENT_STOP,
    D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F, D2D_SIZE_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
    ID2D1LinearGradientBrush, ID2D1PathGeometry, ID2D1RadialGradientBrush, ID2D1SolidColorBrush,
    ID2D1StrokeStyle, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_LARGE, D2D1_ARC_SIZE_SMALL,
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_CAP_STYLE_FLAT, D2D1_CAP_STYLE_ROUND,
    D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED, D2D1_COLOR_SPACE_SRGB, D2D1_DASH_STYLE_CUSTOM,
    D2D1_DASH_STYLE_SOLID, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_ELLIPSE, D2D1_EXTEND_MODE_CLAMP,
    D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_LAYER_OPTIONS1_NONE,
    D2D1_LAYER_PARAMETERS1, D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_LINE_JOIN_ROUND,
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES, D2D1_ROUNDED_RECT,
    D2D1_STROKE_STYLE_PROPERTIES1, D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
    D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGISurface};

pub struct D2DContext {
    pub factory: ID2D1Factory1,
    pub device: ID2D1Device,
    pub context: ID2D1DeviceContext,
    target_bitmap: Option<ID2D1Bitmap1>,
    pub round_stroke_style: ID2D1StrokeStyle,
    pub dashed_stroke_style: ID2D1StrokeStyle,
    pub dotted_stroke_style: ID2D1StrokeStyle,
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

        let round_props = D2D1_STROKE_STYLE_PROPERTIES1 {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            dashCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 10.0,
            dashStyle: D2D1_DASH_STYLE_SOLID,
            dashOffset: 0.0,
            transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
        };
        let round_stroke_style: ID2D1StrokeStyle =
            unsafe { factory.CreateStrokeStyle(&round_props, None)?.into() };

        let dash_props = D2D1_STROKE_STYLE_PROPERTIES1 {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            dashCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 10.0,
            dashStyle: D2D1_DASH_STYLE_CUSTOM,
            dashOffset: 0.0,
            transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
        };
        let dashes = [4.0f32, 3.5f32];
        let dashed_stroke_style: ID2D1StrokeStyle =
            unsafe { factory.CreateStrokeStyle(&dash_props, Some(&dashes))?.into() };

        let dot_props = D2D1_STROKE_STYLE_PROPERTIES1 {
            startCap: D2D1_CAP_STYLE_FLAT,
            endCap: D2D1_CAP_STYLE_FLAT,
            dashCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 10.0,
            dashStyle: D2D1_DASH_STYLE_CUSTOM,
            dashOffset: 0.0,
            transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL,
        };
        let dots = [1.5f32, 2.5f32];
        let dotted_stroke_style: ID2D1StrokeStyle =
            unsafe { factory.CreateStrokeStyle(&dot_props, Some(&dots))?.into() };

        Ok(Self {
            factory,
            device,
            context,
            target_bitmap: None,
            round_stroke_style,
            dashed_stroke_style,
            dotted_stroke_style,
        })
    }

    pub fn set_target_surface(&mut self, surface: &IDXGISurface) -> Result<()> {
        let pixel_format = D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
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

    pub fn create_linear_gradient(
        &self,
        start: (f32, f32),
        end: (f32, f32),
        stops: &[(f32, D2D1_COLOR_F)],
    ) -> Result<ID2D1LinearGradientBrush> {
        let d2d_stops: Vec<D2D1_GRADIENT_STOP> = stops
            .iter()
            .map(|&(pos, color)| D2D1_GRADIENT_STOP { position: pos, color })
            .collect();
        unsafe {
            let stop_collection = self.context.CreateGradientStopCollection(
                &d2d_stops,
                D2D1_COLOR_SPACE_SRGB,
                D2D1_COLOR_SPACE_SRGB,
                D2D1_BUFFER_PRECISION_8BPC_UNORM,
                D2D1_EXTEND_MODE_CLAMP,
                D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED,
            )?;
            let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                startPoint: D2D_POINT_2F { x: start.0, y: start.1 },
                endPoint: D2D_POINT_2F { x: end.0, y: end.1 },
            };
            self.context.CreateLinearGradientBrush(&props, None, &stop_collection)
        }
    }

    pub fn create_radial_gradient(
        &self,
        center: (f32, f32),
        rx: f32,
        ry: f32,
        stops: &[(f32, D2D1_COLOR_F)],
    ) -> Result<ID2D1RadialGradientBrush> {
        let d2d_stops: Vec<D2D1_GRADIENT_STOP> = stops
            .iter()
            .map(|&(pos, color)| D2D1_GRADIENT_STOP { position: pos, color })
            .collect();
        unsafe {
            let stop_collection = self.context.CreateGradientStopCollection(
                &d2d_stops,
                D2D1_COLOR_SPACE_SRGB,
                D2D1_COLOR_SPACE_SRGB,
                D2D1_BUFFER_PRECISION_8BPC_UNORM,
                D2D1_EXTEND_MODE_CLAMP,
                D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED,
            )?;
            let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                center: D2D_POINT_2F { x: center.0, y: center.1 },
                gradientOriginOffset: D2D_POINT_2F { x: 0.0, y: 0.0 },
                radiusX: rx,
                radiusY: ry,
            };
            self.context.CreateRadialGradientBrush(&props, None, &stop_collection)
        }
    }

    pub fn fill_rounded_rect_bitmap(
        &self,
        rect: D2D_RECT_F,
        rx: f32,
        ry: f32,
        bgra_bytes: &[u8],
        width: u32,
        height: u32,
    ) -> bool {
        if bgra_bytes.len() < (width * height * 4) as usize {
            return false;
        }
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: windows::Win32::Graphics::Direct2D::D2D1_BITMAP_OPTIONS_NONE,
            colorContext: std::mem::ManuallyDrop::new(None),
        };
        unsafe {
            if let Ok(bmp) = self.context.CreateBitmap(
                windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U { width, height },
                Some(bgra_bytes.as_ptr() as *const _),
                width * 4,
                &props,
            ) {
                let brush_props = windows::Win32::Graphics::Direct2D::D2D1_BITMAP_BRUSH_PROPERTIES1 {
                    extendModeX: D2D1_EXTEND_MODE_CLAMP,
                    extendModeY: D2D1_EXTEND_MODE_CLAMP,
                    interpolationMode:
                        windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
                };
                let trans_props = windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES {
                    opacity: 1.0,
                    transform: windows::Foundation::Numerics::Matrix3x2 {
                        M11: (rect.right - rect.left) / width as f32,
                        M12: 0.0,
                        M21: 0.0,
                        M22: (rect.bottom - rect.top) / height as f32,
                        M31: rect.left,
                        M32: rect.top,
                    },
                };
                if let Ok(bmp_brush) = self.context.CreateBitmapBrush(
                    &bmp,
                    Some(&brush_props),
                    Some(&trans_props),
                ) {
                    let rounded = D2D1_ROUNDED_RECT {
                        rect,
                        radiusX: rx,
                        radiusY: ry,
                    };
                    self.context.FillRoundedRectangle(&rounded, &bmp_brush);
                    return true;
                }
            }
        }
        false
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

    pub fn fill_rounded_rect_linear(
        &self,
        rect: D2D_RECT_F,
        rx: f32,
        ry: f32,
        brush: &ID2D1LinearGradientBrush,
    ) {
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

    pub fn draw_dashed_rounded_rect(
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
            self.context.DrawRoundedRectangle(
                &rounded,
                brush,
                stroke_width,
                &self.dashed_stroke_style,
            );
        }
    }

    pub fn fill_rect(&self, rect: D2D_RECT_F, brush: &ID2D1SolidColorBrush) {
        unsafe {
            self.context.FillRectangle(&rect, brush);
        }
    }

    pub fn fill_ellipse(&self, center_x: f32, center_y: f32, rx: f32, ry: f32, brush: &ID2D1SolidColorBrush) {
        let ellipse = D2D1_ELLIPSE {
            point: D2D_POINT_2F { x: center_x, y: center_y },
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.FillEllipse(&ellipse, brush);
        }
    }

    pub fn fill_ellipse_radial(
        &self,
        center_x: f32,
        center_y: f32,
        rx: f32,
        ry: f32,
        brush: &ID2D1RadialGradientBrush,
    ) {
        let ellipse = D2D1_ELLIPSE {
            point: D2D_POINT_2F { x: center_x, y: center_y },
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.FillEllipse(&ellipse, brush);
        }
    }

    pub fn draw_ellipse(
        &self,
        center_x: f32,
        center_y: f32,
        rx: f32,
        ry: f32,
        brush: &ID2D1SolidColorBrush,
        stroke_width: f32,
    ) {
        let ellipse = D2D1_ELLIPSE {
            point: D2D_POINT_2F { x: center_x, y: center_y },
            radiusX: rx,
            radiusY: ry,
        };
        unsafe {
            self.context.DrawEllipse(&ellipse, brush, stroke_width, None);
        }
    }

    pub fn draw_line(
        &self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        brush: &ID2D1SolidColorBrush,
        stroke_width: f32,
        round_cap: bool,
    ) {
        unsafe {
            if round_cap {
                self.context.DrawLine(
                    D2D_POINT_2F { x: x0, y: y0 },
                    D2D_POINT_2F { x: x1, y: y1 },
                    brush,
                    stroke_width,
                    &self.round_stroke_style,
                );
            } else {
                self.context.DrawLine(
                    D2D_POINT_2F { x: x0, y: y0 },
                    D2D_POINT_2F { x: x1, y: y1 },
                    brush,
                    stroke_width,
                    None,
                );
            }
        }
    }

    pub fn draw_dotted_line(
        &self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        brush: &ID2D1SolidColorBrush,
        stroke_width: f32,
    ) {
        unsafe {
            self.context.DrawLine(
                D2D_POINT_2F { x: x0, y: y0 },
                D2D_POINT_2F { x: x1, y: y1 },
                brush,
                stroke_width,
                &self.dotted_stroke_style,
            );
        }
    }

    /// Creates and fills the exact Nocan notch silhouette:
    /// flat top edge, concave top-left and top-right ears (`ear_r`),
    /// and rounded bottom-left and bottom-right corners (`corner_r`).
    pub fn create_notch_geometry(
        &self,
        x0: f32,
        y0: f32,
        body_w: f32,
        body_h: f32,
        corner_r: f32,
        ear_r: f32,
    ) -> Result<ID2D1PathGeometry> {
        let x1 = x0 + body_w;
        let y1 = y0 + body_h;
        let r = corner_r.min(body_w * 0.5).min(body_h * 0.5);
        let e = ear_r.min(body_h - r);

        unsafe {
            let path: ID2D1PathGeometry = self.factory.CreatePathGeometry()?.into();
            let sink = path.Open()?;

            sink.BeginFigure(
                D2D_POINT_2F { x: x0 - e, y: y0 },
                D2D1_FIGURE_BEGIN_FILLED,
            );

            // Top flat edge (from left ear tip to right ear tip)
            sink.AddLine(D2D_POINT_2F { x: x1 + e, y: y0 });

            // Right concave ear: arc from (x1 + e, y0) to (x1, y0 + e)
            if e > 0.1 {
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: D2D_POINT_2F { x: x1, y: y0 + e },
                    size: D2D_SIZE_F { width: e, height: e },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
            }

            // Right vertical side
            sink.AddLine(D2D_POINT_2F { x: x1, y: y1 - r });

            // Bottom-right rounded corner
            if r > 0.1 {
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: D2D_POINT_2F { x: x1 - r, y: y1 },
                    size: D2D_SIZE_F { width: r, height: r },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
            }

            // Bottom horizontal edge
            sink.AddLine(D2D_POINT_2F { x: x0 + r, y: y1 });

            // Bottom-left rounded corner
            if r > 0.1 {
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: D2D_POINT_2F { x: x0, y: y1 - r },
                    size: D2D_SIZE_F { width: r, height: r },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
            }

            // Left vertical side
            sink.AddLine(D2D_POINT_2F { x: x0, y: y0 + e });

            // Left concave ear: arc from (x0, y0 + e) to (x0 - e, y0)
            if e > 0.1 {
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: D2D_POINT_2F { x: x0 - e, y: y0 },
                    size: D2D_SIZE_F { width: e, height: e },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
            }

            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            Ok(path)
        }
    }

    pub fn fill_notch_body(
        &self,
        x0: f32,
        y0: f32,
        body_w: f32,
        body_h: f32,
        corner_r: f32,
        ear_r: f32,
        brush: &ID2D1SolidColorBrush,
    ) {
        if let Ok(path) = self.create_notch_geometry(x0, y0, body_w, body_h, corner_r, ear_r) {
            unsafe {
                self.context.FillGeometry(&path, brush, None);
            }
        }
    }

    /// Pushes a Direct2D layer clipped to the notch body silhouette with a given opacity.
    /// Used for smooth spring-animated section crossfades and notch expand/collapse fades.
    pub fn push_clipped_opacity_layer(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        corner_r: f32,
        ear_r: f32,
        opacity: f32,
    ) -> bool {
        if let Ok(geom) = self.create_notch_geometry(body_left, 0.0, body_w, body_h, corner_r, ear_r) {
            let identity = windows::Foundation::Numerics::Matrix3x2 {
                M11: 1.0,
                M12: 0.0,
                M21: 0.0,
                M22: 1.0,
                M31: 0.0,
                M32: 0.0,
            };
            let params = D2D1_LAYER_PARAMETERS1 {
                contentBounds: D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: body_left + body_w + ear_r,
                    bottom: body_h,
                },
                geometricMask: std::mem::ManuallyDrop::new(Some(geom.into())),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: identity,
                opacity: opacity.clamp(0.0, 1.0),
                opacityBrush: std::mem::ManuallyDrop::new(None),
                layerOptions: D2D1_LAYER_OPTIONS1_NONE,
            };
            unsafe {
                self.context.PushLayer(&params, None);
            }
            let _ = std::mem::ManuallyDrop::into_inner(params.geometricMask);
            return true;
        }
        false
    }

    pub fn pop_layer(&self) {
        unsafe {
            self.context.PopLayer();
        }
    }

    /// Draws the authentic Nocan `.ambient` bottom-up masked horizontal gradient glow
    /// (used by both the Nocan Mascot bar and the Lyrics mode ambient background).
    /// Matches `nocan.original.exe` (`0x140038f0f` & `0x140039002`):
    /// - Horizontal gradient across `[body_left + shift_x .. body_left + body_w + shift_x]`
    /// - Masked vertically by the 6-stop alpha gradient from `y = body_h` (alpha 1.0)
    ///   up to `y = body_h * (1.0 - height_ratio)` (alpha 0.0)
    /// - Clipped to the notch body rounded-bottom silhouette (`corner_r`).
    pub fn draw_ambient_glow(
        &self,
        body_left: f32,
        body_w: f32,
        body_h: f32,
        corner_r: f32,
        height_ratio: f32,
        layer_opacity: f32,
        shift_x: f32,
        horiz_stops: &[(f32, D2D1_COLOR_F)],
    ) {
        let top_y = body_h * (1.0 - height_ratio.clamp(0.1, 1.0));
        let mask_stops = [
            (0.00, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.00 }),
            (0.15, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.80 }),
            (0.30, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.55 }),
            (0.50, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.28 }),
            (0.70, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.10 }),
            (1.00, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.00 }),
        ];

        if let (Ok(geom), Ok(mask_brush), Ok(horiz_brush)) = (
            self.create_notch_geometry(body_left, 0.0, body_w, body_h, corner_r, 0.0),
            self.create_linear_gradient((0.0, body_h), (0.0, top_y), &mask_stops),
            self.create_linear_gradient(
                (body_left + shift_x, 0.0),
                (body_left + body_w + shift_x, 0.0),
                horiz_stops,
            ),
        ) {
            let identity = windows::Foundation::Numerics::Matrix3x2 {
                M11: 1.0,
                M12: 0.0,
                M21: 0.0,
                M22: 1.0,
                M31: 0.0,
                M32: 0.0,
            };
            let params = D2D1_LAYER_PARAMETERS1 {
                contentBounds: D2D_RECT_F {
                    left: body_left,
                    top: 0.0,
                    right: body_left + body_w,
                    bottom: body_h,
                },
                geometricMask: std::mem::ManuallyDrop::new(Some(geom.into())),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: identity,
                opacity: layer_opacity.clamp(0.0, 1.0),
                opacityBrush: std::mem::ManuallyDrop::new(Some(mask_brush.into())),
                layerOptions: D2D1_LAYER_OPTIONS1_NONE,
            };
            unsafe {
                self.context.PushLayer(&params, None);
                let fill_rect = D2D_RECT_F {
                    left: body_left,
                    top: 0.0,
                    right: body_left + body_w,
                    bottom: body_h,
                };
                self.context.FillRectangle(&fill_rect, &horiz_brush);
                self.context.PopLayer();
            }
            let _ = std::mem::ManuallyDrop::into_inner(params.geometricMask);
            let _ = std::mem::ManuallyDrop::into_inner(params.opacityBrush);
        }
    }

    /// Draws a circular progress ring (track + partial arc starting at -90 deg).
    pub fn draw_progress_ring(
        &self,
        cx: f32,
        cy: f32,
        radius: f32,
        stroke_width: f32,
        progress: f32,
        track_brush: &ID2D1SolidColorBrush,
        arc_brush: &ID2D1SolidColorBrush,
    ) {
        self.draw_ellipse(cx, cy, radius, radius, track_brush, stroke_width);

        let p = progress.clamp(0.0, 1.0);
        if p <= 0.005 {
            return;
        }
        if p >= 0.995 {
            self.draw_ellipse(cx, cy, radius, radius, arc_brush, stroke_width);
            return;
        }

        let start_angle = -std::f32::consts::FRAC_PI_2;
        let end_angle = start_angle + p * std::f32::consts::TAU;

        let start_pt = D2D_POINT_2F {
            x: cx + radius * start_angle.cos(),
            y: cy + radius * start_angle.sin(),
        };
        let end_pt = D2D_POINT_2F {
            x: cx + radius * end_angle.cos(),
            y: cy + radius * end_angle.sin(),
        };

        unsafe {
            if let Ok(path) = self.factory.CreatePathGeometry() {
                if let Ok(sink) = path.Open() {
                    sink.BeginFigure(start_pt, D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: end_pt,
                        size: D2D_SIZE_F {
                            width: radius,
                            height: radius,
                        },
                        rotationAngle: 0.0,
                        sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                        arcSize: if p > 0.5 {
                            D2D1_ARC_SIZE_LARGE
                        } else {
                            D2D1_ARC_SIZE_SMALL
                        },
                    });
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    let _ = sink.Close();
                    self.context
                        .DrawGeometry(&path, arc_brush, stroke_width, &self.round_stroke_style);
                }
            }
        }
    }

    /// Draws an open circular arc from `start_deg` to `end_deg` (clockwise, 0 deg = +X right, 90 deg = +Y down).
    pub fn draw_arc_segment(
        &self,
        cx: f32,
        cy: f32,
        radius: f32,
        start_deg: f32,
        end_deg: f32,
        stroke_width: f32,
        brush: &ID2D1SolidColorBrush,
    ) {
        let sweep_deg = (end_deg - start_deg).clamp(0.0, 359.9);
        if sweep_deg <= 0.1 {
            return;
        }
        let s_rad = start_deg.to_radians();
        let e_rad = (start_deg + sweep_deg).to_radians();
        let start_pt = D2D_POINT_2F {
            x: cx + radius * s_rad.cos(),
            y: cy + radius * s_rad.sin(),
        };
        let end_pt = D2D_POINT_2F {
            x: cx + radius * e_rad.cos(),
            y: cy + radius * e_rad.sin(),
        };
        unsafe {
            if let Ok(path) = self.factory.CreatePathGeometry() {
                if let Ok(sink) = path.Open() {
                    sink.BeginFigure(start_pt, D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: end_pt,
                        size: D2D_SIZE_F {
                            width: radius,
                            height: radius,
                        },
                        rotationAngle: 0.0,
                        sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                        arcSize: if sweep_deg > 180.0 {
                            D2D1_ARC_SIZE_LARGE
                        } else {
                            D2D1_ARC_SIZE_SMALL
                        },
                    });
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    let _ = sink.Close();
                    self.context
                        .DrawGeometry(&path, brush, stroke_width, &self.round_stroke_style);
                }
            }
        }
    }

    /// Draws a stroked open polyline with round caps and joins.
    pub fn draw_polyline(
        &self,
        points: &[(f32, f32)],
        brush: &ID2D1SolidColorBrush,
        stroke_width: f32,
        closed: bool,
    ) {
        if points.len() < 2 {
            return;
        }
        unsafe {
            if let Ok(path) = self.factory.CreatePathGeometry() {
                if let Ok(sink) = path.Open() {
                    sink.BeginFigure(
                        D2D_POINT_2F {
                            x: points[0].0,
                            y: points[0].1,
                        },
                        D2D1_FIGURE_BEGIN_HOLLOW,
                    );
                    for &(x, y) in &points[1..] {
                        sink.AddLine(D2D_POINT_2F { x, y });
                    }
                    sink.EndFigure(if closed {
                        D2D1_FIGURE_END_CLOSED
                    } else {
                        D2D1_FIGURE_END_OPEN
                    });
                    let _ = sink.Close();
                    self.context
                        .DrawGeometry(&path, brush, stroke_width, &self.round_stroke_style);
                }
            }
        }
    }

    /// Fills a closed polygon defined by a slice of points.
    pub fn fill_polygon(&self, points: &[(f32, f32)], brush: &ID2D1SolidColorBrush) {
        if points.len() < 3 {
            return;
        }
        unsafe {
            if let Ok(path) = self.factory.CreatePathGeometry() {
                if let Ok(sink) = path.Open() {
                    sink.BeginFigure(
                        D2D_POINT_2F {
                            x: points[0].0,
                            y: points[0].1,
                        },
                        D2D1_FIGURE_BEGIN_FILLED,
                    );
                    for &(x, y) in &points[1..] {
                        sink.AddLine(D2D_POINT_2F { x, y });
                    }
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _ = sink.Close();
                    self.context.FillGeometry(&path, brush, None);
                }
            }
        }
    }

    /// Draws the 5-bar animated sound wave / equalizer (`.bars`) centered at `(cx, cy)`.
    pub fn draw_equalizer_bars(
        &self,
        cx: f32,
        cy: f32,
        is_playing: bool,
        time_sec: f32,
        brush: &ID2D1SolidColorBrush,
    ) {
        let bar_w = 2.2f32;
        let gap = 2.0f32;
        let max_h = 14.0f32;
        let total_w = 5.0 * bar_w + 4.0 * gap;
        let start_x = cx - total_w * 0.5;
        let delays = [0.0f32, -0.3f32, -0.6f32, -0.15f32, -0.45f32];
        let idle_heights = [0.35f32, 0.72f32, 0.50f32, 0.90f32, 0.42f32];

        for i in 0..5 {
            let ratio = if is_playing {
                // CSS: animation: eq .9s ease-in-out infinite alternate (30% to 100%)
                let t = (time_sec - delays[i]) / 0.9;
                let phase = (t * std::f32::consts::PI).sin().abs();
                0.30 + 0.70 * phase
            } else {
                idle_heights[i]
            };
            let h = (max_h * ratio).max(3.0);
            let x = start_x + (i as f32) * (bar_w + gap);
            let rect = D2D_RECT_F {
                left: x,
                top: cy - h * 0.5,
                right: x + bar_w,
                bottom: cy + h * 0.5,
            };
            self.fill_rounded_rect(rect, 1.1, 1.1, brush);
        }
    }

    /// Draws a water droplet icon centered at `(cx, cy)` with radius/scale `s`.
    pub fn fill_droplet(&self, cx: f32, cy: f32, s: f32, brush: &ID2D1SolidColorBrush) {
        // SVG: M12 3s6 6.5 6 11a6 6 0 0 1-12 0c0-4.5 6-11 6-11z (viewBox 0 0 24 24)
        let scale = s / 12.0;
        let tx = |x: f32| cx + (x - 12.0) * scale;
        let ty = |y: f32| cy + (y - 12.5) * scale;

        unsafe {
            if let Ok(path) = self.factory.CreatePathGeometry() {
                if let Ok(sink) = path.Open() {
                    sink.BeginFigure(
                        D2D_POINT_2F {
                            x: tx(12.0),
                            y: ty(3.0),
                        },
                        D2D1_FIGURE_BEGIN_FILLED,
                    );
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: D2D_POINT_2F {
                            x: tx(15.0),
                            y: ty(6.5),
                        },
                        point2: D2D_POINT_2F {
                            x: tx(18.0),
                            y: ty(10.5),
                        },
                        point3: D2D_POINT_2F {
                            x: tx(18.0),
                            y: ty(14.0),
                        },
                    });
                    sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: D2D_POINT_2F {
                            x: tx(6.0),
                            y: ty(14.0),
                        },
                        size: D2D_SIZE_F {
                            width: 6.0 * scale,
                            height: 6.0 * scale,
                        },
                        rotationAngle: 0.0,
                        sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                        arcSize: D2D1_ARC_SIZE_SMALL,
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: D2D_POINT_2F {
                            x: tx(6.0),
                            y: ty(10.5),
                        },
                        point2: D2D_POINT_2F {
                            x: tx(9.0),
                            y: ty(6.5),
                        },
                        point3: D2D_POINT_2F {
                            x: tx(12.0),
                            y: ty(3.0),
                        },
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    let _ = sink.Close();
                    self.context.FillGeometry(&path, brush, None);
                }
            }
        }
    }
}
