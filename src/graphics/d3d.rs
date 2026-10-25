use windows::core::{Interface, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIDevice, IDXGIFactory2, IDXGISurface, IDXGISwapChain1,
    DXGI_CREATE_FACTORY_FLAGS, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
    DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

pub struct D3DContext {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub dxgi_device: IDXGIDevice,
    pub factory: IDXGIFactory2,
    pub swapchain: IDXGISwapChain1,
    pub width: u32,
    pub height: u32,
}

impl D3DContext {
    pub fn new(_hwnd: HWND, width: u32, height: u32) -> Result<Self> {
        let creation_flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

        let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
        let mut feature_level = D3D_FEATURE_LEVEL_11_0;

        let mut device_opt = None;
        let mut context_opt = None;

        // Try Hardware first, fallback to WARP
        let hr = unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                creation_flags,
                Some(&feature_levels),
                windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION,
                Some(&mut device_opt),
                Some(&mut feature_level),
                Some(&mut context_opt),
            )
        };

        let (device, context) = if hr.is_ok() {
            (device_opt.unwrap(), context_opt.unwrap())
        } else {
            let mut dev = None;
            let mut ctx = None;
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_WARP,
                    None,
                    creation_flags,
                    Some(&feature_levels),
                    windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION,
                    Some(&mut dev),
                    Some(&mut feature_level),
                    Some(&mut ctx),
                )?;
            }
            (dev.unwrap(), ctx.unwrap())
        };

        let dxgi_device: IDXGIDevice = device.cast()?;
        let factory: IDXGIFactory2 = unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))? };

        let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: windows::Win32::Foundation::BOOL(0),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: windows::Win32::Graphics::Dxgi::DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            Flags: 0,
        };

        // Create swapchain for composition
        let swapchain = unsafe {
            factory.CreateSwapChainForComposition(&device, &swapchain_desc, None)?
        };

        Ok(Self {
            device,
            context,
            dxgi_device,
            factory,
            swapchain,
            width,
            height,
        })
    }

    pub fn get_surface(&self) -> Result<IDXGISurface> {
        unsafe { self.swapchain.GetBuffer(0) }
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if self.width == width && self.height == height {
            return Ok(());
        }
        self.width = width.max(1);
        self.height = height.max(1);

        unsafe {
            self.swapchain.ResizeBuffers(
                2,
                self.width,
                self.height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                windows::Win32::Graphics::Dxgi::DXGI_SWAP_CHAIN_FLAG(0),
            )?;
        }
        Ok(())
    }

    pub fn present(&self) -> Result<()> {
        unsafe {
            self.swapchain.Present(1, windows::Win32::Graphics::Dxgi::DXGI_PRESENT(0)).ok()
        }
    }
}
