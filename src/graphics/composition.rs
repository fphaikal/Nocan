use windows::core::Result;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain1;

pub struct CompositionTarget {
    pub device: IDCompositionDevice,
    pub target: IDCompositionTarget,
    pub visual: IDCompositionVisual,
}

impl CompositionTarget {
    pub fn new(hwnd: HWND, dxgi_device: &IDXGIDevice, swapchain: &IDXGISwapChain1) -> Result<Self> {
        let device: IDCompositionDevice = unsafe {
            DCompositionCreateDevice(dxgi_device)?
        };

        let target = unsafe { device.CreateTargetForHwnd(hwnd, false)? };
        let visual = unsafe { device.CreateVisual()? };

        unsafe {
            visual.SetContent(swapchain)?;
            target.SetRoot(&visual)?;
            device.Commit()?;
        }

        Ok(Self {
            device,
            target,
            visual,
        })
    }
}
