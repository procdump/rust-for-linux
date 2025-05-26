// SPDX-License-Identifier: GPL-2.0

// Copyright (c) 2025 Boris Astardzhiev <boris.astardzhiev@gmail.com>

//! Net namespaces.
//!
//! C header: [`include/net/net_namespace.h`](srctree/include/net/net_namespace.h) and
//! [`include/linux/nsproxy.h`](srctree/include/linux/nsproxy.h)

use crate::{
    bindings, pr_info,
    types::{AlwaysRefCounted, Opaque},
};
use core::ptr;

/// Wraps the kernel's `struct net`. Thread safe.
///
/// This structure represents the Rust abstraction for a C `struct net`. This
/// implementation abstracts the usage of an already existing C `struct net` within Rust
/// code that we get passed from the C side.
#[repr(transparent)]
pub struct NetNamespace {
    inner: Opaque<bindings::net>,
}

impl NetNamespace {
    /// Returns a raw pointer to the inner C struct.
    #[inline]
    pub fn as_ptr(&self) -> *mut bindings::net {
        self.inner.get()
    }

    /// Creates a reference to a [`NetNamespace`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid and remains valid for the lifetime of the
    /// returned [`NetNamespace`] reference.
    pub unsafe fn from_ptr<'a>(ptr: *const bindings::net) -> &'a Self {
        // SAFETY: The safety requirements guarantee the validity of the dereference, while the
        // `NetNamespace` type being transparent makes the cast ok.
        unsafe { &*ptr.cast() }
    }
}

// SAFETY: Instances of `NetNamespace` are always reference-counted.
unsafe impl AlwaysRefCounted for NetNamespace {
    #[inline]
    fn inc_ref(&self) {
        pr_info!("Incrementing ref to netnamespace!\n");
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { bindings::get_net_namespace(self.as_ptr()) };
    }

    #[inline]
    unsafe fn dec_ref(obj: ptr::NonNull<NetNamespace>) {
        pr_info!("Decrementing ref to netnamespace!\n");
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::put_net_namespace(obj.cast().as_ptr()) }
    }
}

// SAFETY:
// - `NetNamespace::dec_ref` can be called from any thread.
// - It is okay to send ownership of `NetNamespace` across thread boundaries.
unsafe impl Send for NetNamespace {}

// SAFETY: It's OK to access `NetNamespace` through shared references from other threads because
// we're either accessing properties that don't change or that are properly synchronised by C code.
unsafe impl Sync for NetNamespace {}
