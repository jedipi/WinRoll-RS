use windows::{
    Win32::{
        Foundation::POINT,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
        UI::Accessibility::*,
    },
    core::Result,
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct Automation {
    client: IUIAutomation2,
    cache: IUIAutomationCacheRequest,
    walker: IUIAutomationTreeWalker,
    // Drop the interfaces before uninitializing their thread's COM apartment.
    _apartment: Apartment,
}

impl Automation {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let client: IUIAutomation2 =
                CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
            // An unresponsive accessibility provider must not hold up window recovery.
            client.SetConnectionTimeout(100)?;
            client.SetTransactionTimeout(100)?;
            let cache = client.CreateCacheRequest()?;
            for property in [
                UIA_AutomationIdPropertyId,
                UIA_ProcessIdPropertyId,
                UIA_ControlTypePropertyId,
                UIA_IsEnabledPropertyId,
                UIA_IsOffscreenPropertyId,
                UIA_BoundingRectanglePropertyId,
            ] {
                cache.AddProperty(property)?;
            }
            let walker = client.RawViewWalker()?;
            Ok(Self {
                client,
                cache,
                walker,
                _apartment: apartment,
            })
        }
    }

    fn fork_close(&self, point: POINT, pid: u32) -> Result<bool> {
        unsafe {
            let mut element = self.client.ElementFromPointBuildCache(point, &self.cache)?;
            // The pointer may land on an image inside the Close button.
            for _ in 0..6 {
                if element.CachedProcessId()? as u32 != pid {
                    return Ok(false);
                }
                if element.CachedAutomationId()? == "PART_CloseButton" {
                    let bounds = element.CachedBoundingRectangle()?;
                    return Ok(element.CachedControlType()? == UIA_ButtonControlTypeId
                        && element.CachedIsEnabled()?.as_bool()
                        && !element.CachedIsOffscreen()?.as_bool()
                        && point.x >= bounds.left
                        && point.x < bounds.right
                        && point.y >= bounds.top
                        && point.y < bounds.bottom);
                }
                element = self
                    .walker
                    .GetParentElementBuildCache(&element, &self.cache)?;
            }
            Ok(false)
        }
    }
}

thread_local! {
    static AUTOMATION: Option<Automation> = Automation::new().ok();
}

pub fn initialize() {
    AUTOMATION.with(|_| {});
}

pub fn fork_close(x: i32, y: i32, pid: u32) -> bool {
    AUTOMATION.with(|automation| {
        automation
            .as_ref()
            .and_then(|automation| automation.fork_close(POINT { x, y }, pid).ok())
            .unwrap_or(false)
    })
}
