use core::ops::{Deref, DerefMut};
use kernel::bindings::{
    dev_queue_xmit, kfree_skb, net_device, netif_rx, sk_buff, skb_copy, GFP_ATOMIC,
};
use kernel::types::Opaque;

pub(crate) struct SkBuffOwned<'a> {
    skb: Option<&'a mut SkBuff>,
}

impl<'a> SkBuffOwned<'a> {
    #[allow(dead_code)]
    pub(crate) fn new(skb: &'a mut SkBuff) -> Self {
        Self { skb: Some(skb) }
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn from_raw(skb: *mut sk_buff) -> Self {
        unsafe {
            Self {
                skb: Some(SkBuff::from_raw(skb)),
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn dev_queue_xmit(mut self) -> i32 {
        let inner = self.skb.take().unwrap();
        let skb = inner.get_raw();
        unsafe { dev_queue_xmit(skb) }
    }

    #[allow(dead_code)]
    pub(crate) fn netif_rx(mut self) -> i32 {
        let inner = self.skb.take().unwrap();
        let skb = inner.get_raw();
        unsafe { netif_rx(skb) }
    }
}

impl<'a> Drop for SkBuffOwned<'a> {
    fn drop(&mut self) {
        if let Some(inner) = &self.skb {
            unsafe {
                kfree_skb(inner.get_raw());
            }
        }
    }
}

impl<'a> Deref for SkBuffOwned<'a> {
    type Target = SkBuff;
    fn deref(&self) -> &Self::Target {
        self.skb.as_ref().unwrap()
    }
}

impl<'a> DerefMut for SkBuffOwned<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.skb.as_mut().unwrap()
    }
}

#[repr(transparent)]
pub(crate) struct SkBuff(Opaque<sk_buff>);

impl SkBuff {
    /// Creates a new [`SkBuff`] instance from a raw pointer.
    ///
    /// # Safety
    ///
    /// For the duration of 'a, the pointer must point at a valid `sk_buff`,
    /// and the caller must be in a context where all methods defined on this struct
    /// are safe to call.
    pub(crate) unsafe fn from_raw<'a>(ptr: *mut sk_buff) -> &'a mut Self {
        // CAST: `Self` is a `repr(transparent)` wrapper around `sk_buff`.
        let ptr = ptr.cast::<Self>();
        // SAFETY: by the function requirements the pointer is valid and we have unique access for
        // the duration of `'a`.
        unsafe { &mut *ptr }
    }

    pub(crate) fn get_raw(&self) -> *mut sk_buff {
        self.0.get()
    }

    #[allow(dead_code)]
    pub(crate) fn undo_skb_pull(&mut self, how_many: usize) {
        let skb = self.get_raw();
        unsafe {
            (*skb).data = (*skb).data.offset(-(how_many as isize));
            (*skb).len += how_many as u32;
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_pkt_type(&self) -> u32 {
        let skb = self.get_raw();
        unsafe {
            let pkt_type = (*skb).__bindgen_anon_5.__bindgen_anon_1.as_ref().pkt_type() as u32;
            // let offset = -(ETH_HLEN as isize);
            // pr_info!("pkt_type: {}\n", pkt_type);
            // pr_info!("skb->data: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}\n",
            //      *((*skb).data.offset(offset+0)), *((*skb).data.offset(offset+1)), *((*skb).data.offset(offset+2)),
            //      *((*skb).data.offset(offset+3)), *((*skb).data.offset(offset+4)), *((*skb).data.offset(offset+5)),
            //      *((*skb).data.offset(offset+6)), *((*skb).data.offset(offset+7)));

            pkt_type
        }
    }

    #[allow(dead_code)]
    // for a lifetime of 'a give me a reference to the cloned skb
    pub(crate) fn clone<'a, 'b>(&'b self) -> SkBuffOwned<'a> {
        unsafe {
            let skb = self.get_raw();
            let nskb = skb_copy(skb, GFP_ATOMIC);
            SkBuffOwned::from_raw(nskb)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_dev(&mut self, dev: *mut net_device) {
        let skb = self.get_raw();
        unsafe {
            (*skb)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .__bindgen_anon_1
                .dev = dev;
        }
    }
}
