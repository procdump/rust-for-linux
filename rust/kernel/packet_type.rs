// SPDX-License-Identifier: GPL-2.0

// Copyright (c) 2025 Boris Astardzhiev <boris.astardzhiev@gmail.com>

//! Packet types.
//!
//! Abstraction for having a callback for packets matching a specified ethernet protocol.

use core::marker::PhantomData;
use core::{mem, pin::Pin};

use crate::alloc::AllocError;
use crate::prelude::GFP_KERNEL;
use crate::{
    alloc::KBox,
    types::{ForeignOwnable, Opaque},
};
use bindings::{dev_add_pack, dev_remove_pack};
use kernel::bindings::{net_device, packet_type, sk_buff};

/// Wraps kernel's `packet_type` to have a callback for packets matching a specified ethernet protocol.
///
/// This structure represents the Rust abstraction for a C `struct packet_type`.
pub struct PacketType<T>
where
    T: ForeignOwnable,
{
    packet_type: Pin<KBox<Opaque<bindings::packet_type>>>,
    _marker: PhantomData<T>,
}

impl<T: ForeignOwnable> PacketType<T> {
    /// Create a [`PacketType`] instance for a specified callback embedding some private data `T`
    pub unsafe fn new(
        ether_type: u16,
        pkt_handler: unsafe extern "C" fn(
            skb: *mut sk_buff,
            dev_in: *mut net_device,
            packet_type: *mut packet_type,
            orig_dev: *mut net_device,
        ) -> i32,
        private: T,
    ) -> Result<Self, AllocError> {
        // SAFETY: It seems legitimate to have this structure zeroed
        // and ready for attaching with `dev_add_pack`.
        let packet_type: bindings::packet_type = unsafe { mem::zeroed() };
        let packet_type = KBox::pin(Opaque::new(packet_type), GFP_KERNEL)?;
        let pt = packet_type.get();
        // SAFETY: Populate the fields.
        unsafe {
            (*pt).type_ = ether_type.to_be();
            (*pt).func = Some(pkt_handler);
            (*pt).af_packet_priv = private.into_foreign();
            dev_add_pack(pt);
        }

        Ok(Self {
            packet_type,
            _marker: PhantomData,
        })
    }

    /// A method for getting a borrow to the private
    /// data embedded in the main `packet_type` structure
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid and remains valid for the lifetime 'a
    pub unsafe fn borrow_private<'a>(
        ptr: *const bindings::packet_type,
    ) -> <T as ForeignOwnable>::Borrowed<'a> {
        unsafe { <T as ForeignOwnable>::borrow((*ptr).af_packet_priv) }
    }

    /// A method for getting a mutable borrow to the private
    /// data embedded in the main `packet_type` structure
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid and remains valid for the lifetime 'a
    pub unsafe fn borrow_private_mut<'a>(
        ptr: *const bindings::packet_type,
    ) -> <T as ForeignOwnable>::BorrowedMut<'a> {
        unsafe { <T as ForeignOwnable>::borrow_mut((*ptr).af_packet_priv) }
    }
}

impl<T: ForeignOwnable> Drop for PacketType<T> {
    fn drop(&mut self) {
        let pt = self.packet_type.get();
        // SAFETY: Detach the callback first and then obtain back
        // the embedded private data.
        unsafe {
            let private = (*pt).af_packet_priv;
            dev_remove_pack(pt);
            let _priv: T = ForeignOwnable::from_foreign(private);
        }
    }
}

// SAFETY: It is okay to send ownership of `PacketType` across thread boundaries.
unsafe impl<T: ForeignOwnable> Send for PacketType<T> {}

// SAFETY: It's OK to access `PacketType` through shared references from other threads because
// we're either accessing properties that don't change or that are properly synchronised by C code.
unsafe impl<T: ForeignOwnable> Sync for PacketType<T> {}
