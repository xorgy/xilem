// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::UpdateCtx;

/// The radius of a widget's box corners.
#[derive(Clone, Copy, Debug)]
pub struct CornerRadius {
    pub radius: f64,
}

impl CornerRadius {
    pub(crate) fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_layout();
    }
}
