use core::mem::MaybeUninit;
use kernel::bindings::{get_net_track, net, netns_tracker, put_net_track, GFP_KERNEL};
use kernel::prelude::*;

pub(crate) struct NetNamespace {
    net: *mut net,
    #[allow(dead_code)]
    tracker: Pin<&'static mut MaybeUninit<netns_tracker>>,
}

impl NetNamespace {
    pub(crate) fn new(
        namespace: *mut net,
        tracker: &'static mut MaybeUninit<netns_tracker>,
    ) -> Self {
        let mut netns_tracker = Pin::static_mut(tracker);
        let net = unsafe { get_net_track(namespace, (*netns_tracker).as_mut_ptr(), GFP_KERNEL) };
        Self {
            net,
            tracker: netns_tracker,
        }
    }

    pub(crate) fn get_net(&self) -> *mut net {
        self.net
    }
}

impl Drop for NetNamespace {
    fn drop(&mut self) {
        unsafe { put_net_track(self.net, (*self.tracker).as_mut_ptr()) };
        pr_info!("NetNamespace dropped\n");
    }
}
