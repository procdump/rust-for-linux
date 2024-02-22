use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use kernel::bindings::{get_net_track, net, netns_tracker, put_net_track, GFP_KERNEL};
use kernel::prelude::*;
use kernel::sync::Arc;

pub(crate) struct NetNamespace {
    net: *mut net,
    #[allow(dead_code)]
    tracker: Arc<NetNsTracker>,
}

impl NetNamespace {
    pub(crate) fn new(namespace: *mut net, tracker: Arc<NetNsTracker>) -> Self {
        pr_info!("Acquiring netns_tracker: {:p}\n", tracker.get_raw());
        let net = unsafe { get_net_track(namespace, tracker.get_raw(), GFP_KERNEL) };
        Self { net, tracker }
    }

    pub(crate) fn get_net(&self) -> *mut net {
        self.net
    }
}

impl Drop for NetNamespace {
    fn drop(&mut self) {
        pr_info!(
            "NetNamespace dropped, tracker: {:p}\n",
            self.tracker.get_raw()
        );
        unsafe { put_net_track(self.net, self.tracker.get_raw()) };
    }
}

#[pin_data]
pub struct NetNsTracker {
    #[pin]
    tracker: UnsafeCell<MaybeUninit<netns_tracker>>,
}

impl NetNsTracker {
    pub(crate) fn new() -> impl PinInit<Self> {
        pin_init!(Self {
            tracker: unsafe { UnsafeCell::new(MaybeUninit::zeroed().assume_init()) }
        })
    }

    pub(crate) fn get_raw(&self) -> *mut netns_tracker {
        unsafe { (*self.tracker.get()).as_mut_ptr() }
    }
}
