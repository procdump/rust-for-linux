// SPDX-License-Identifier: GPL-2.0

// Copyright (c) 2025 Boris Astardzhiev <boris.astardzhiev@gmail.com>

//! Net devices.
//!
//! C header: [`include/linux/net_device.h`](srctree/include/linux/net_device.h)

use crate::{
    bindings,
    net_namespace::NetNamespace,
    pr_info,
    str::CStr,
    types::{ARef, AlwaysRefCounted, Opaque},
};
use core::ptr;

/// Wraps the kernel's `struct net_device`. Thread safe.
///
/// This structure represents the Rust abstraction for a C `struct net_device`. This
/// implementation abstracts the usage of an already existing C `struct net_device` within Rust
/// code that we get passed from the C side.
#[repr(transparent)]
pub struct NetDevice {
    inner: Opaque<bindings::net_device>,
}

impl NetDevice {
    /// Returns a raw pointer to the inner C struct.
    #[inline]
    pub fn as_ptr(&self) -> *mut bindings::net_device {
        self.inner.get()
    }

    /// Creates a reference to a [`NetDevice`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid and remains valid for the lifetime of the
    /// returned [`NetDevice`] reference.
    pub unsafe fn from_ptr<'a>(ptr: *const bindings::net_device) -> &'a Self {
        // SAFETY: The safety requirements guarantee the validity of the dereference, while the
        // `NetDevice` type being transparent makes the cast ok.
        unsafe { &*ptr.cast() }
    }

    /// Try to grab a [`NetDevice`] in a namespace.
    pub fn get_by_name(ns: &NetNamespace, name: &CStr) -> Option<ARef<NetDevice>> {
        // SAFETY: It's safe to call `get_net_device_by_name` as long as we have a namespace and a name.
        let ptr = unsafe { bindings::get_net_device_by_name(ns.as_ptr(), name.as_char_ptr()) };
        if ptr.is_null() {
            None
        } else {
            // SAFETY: `ptr` is valid by the safety requirements of this function. And we own a
            // reference count via `dev_get_by_name()`.
            // CAST: `Self` is a `repr(transparent)` wrapper around `bindings::net_device`.
            Some(unsafe { ARef::from_raw(ptr::NonNull::new_unchecked(ptr.cast::<NetDevice>())) })
        }
    }

    /// Get the name of this net device.
    pub fn name(&self) -> &CStr {
        let ptr = self.inner.get();
        // SAFETY: ptr is a valid pointer to a `struct net_device`.
        unsafe { CStr::from_char_ptr((*ptr).name.as_ptr()) }
    }
}

// SAFETY: Instances of `NetDevice` are always reference-counted.
unsafe impl AlwaysRefCounted for NetDevice {
    #[inline]
    fn inc_ref(&self) {
        pr_info!("Incrementing ref to net_device!\n");
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { bindings::get_net_device(self.as_ptr()) };
    }

    #[inline]
    unsafe fn dec_ref(obj: ptr::NonNull<NetDevice>) {
        pr_info!("Decrementing ref to net_device!\n");
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::put_net_device(obj.cast().as_ptr()) }
    }
}

// SAFETY:
// - `NetDevice::dec_ref` can be called from any thread.
// - It is okay to send ownership of `NetDevice` across thread boundaries.
unsafe impl Send for NetDevice {}

// SAFETY: It's OK to access `NetDevice` through shared references from other threads because
// we're either accessing properties that don't change or that are properly synchronised by C code.
unsafe impl Sync for NetDevice {}
