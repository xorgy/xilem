// Copyright 2022 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A simple scroll view.
//!
//! There's a lot more functionality in the Druid version, including
//! control over scrolling axes, ability to scroll to content, etc.

use crate::Axis;
use xilem_core::Id;

use vello::kurbo::{Affine, Size, Vec2};
use vello::peniko::Mix;
use vello::Scene;

use super::{BoxConstraints, ScrollDelta, Widget};

use super::{AccessCx, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, UpdateCx};

// This number can be related to a platform detail, for example
// on Windows there is SPI_GETWHEELSCROLLLINES
// This number should also be configurable on a given scroll context.
// When scroll gesture handling is hoisted up outside of the widget layer, as it ultimately must be,
// this value will be abstracted away for most users.
const LINE_HEIGHT: f64 = 53.0;

pub struct ScrollView {
    child: Pod,
    offset: f64,
}

impl ScrollView {
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollView {
            child: Pod::new(child, Id::next()),
            offset: 0.0,
        }
    }

    pub fn child_mut(&mut self) -> &mut Pod {
        &mut self.child
    }
}

// TODO: scroll bars
impl Widget for ScrollView {
    fn event(&mut self, cx: &mut EventCx, event: &Event) {
        // Pass event through to child, adjusting the coordinates of mouse events
        // by the scroll offset first.
        // TODO: scroll wheel + click-drag on scroll bars
        let offset = Vec2::new(0.0, self.offset);
        let child_event = match event {
            Event::MouseDown(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseDown(mouse_event)
            }
            Event::MouseUp(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseUp(mouse_event)
            }
            Event::MouseMove(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseMove(mouse_event)
            }
            Event::MouseWheel(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                Event::MouseWheel(mouse_event)
            }
            _ => event.clone(),
        };

        self.child.event(cx, &child_event);

        // Handle scroll wheel events
        if !cx.is_handled() {
            if let Event::MouseWheel(mouse_event) = event {
                let max_offset = (self.child.size().height - cx.size().height).max(0.0);
                // A positive wheel_delta y means our content needs to "move" down (i.e. scroll up), which
                // means the offset needs to *decrease*, because offset increases as you scroll further down
                let y_delta = match mouse_event.wheel_delta {
                    Some(ScrollDelta::Precise(Vec2 { y, .. })) => -y,
                    Some(ScrollDelta::Lines(_, y)) => -y as f64 * LINE_HEIGHT,
                    None => 0.0,
                };
                let new_offset = (self.offset + y_delta).clamp(0.0, max_offset);
                if new_offset != self.offset {
                    self.offset = new_offset;
                    cx.set_handled(true);
                    cx.request_paint();
                }
            }
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.child.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.child.update(cx);
    }

    fn compute_max_intrinsic(&mut self, axis: Axis, cx: &mut LayoutCx, bc: &BoxConstraints) -> f64 {
        match axis {
            Axis::Horizontal => {
                if bc.min().width.is_sign_negative() {
                    0.0
                } else {
                    let length =
                        self.child
                            .compute_max_intrinsic(axis, cx, &bc.unbound_max_height());
                    length.min(bc.max().width).max(bc.min().width)
                }
            }
            Axis::Vertical => {
                if bc.min().height.is_sign_negative() {
                    0.0
                } else {
                    let length =
                        self.child
                            .compute_max_intrinsic(axis, cx, &bc.unbound_max_height());
                    length.min(bc.max().height).max(bc.min().height)
                }
            }
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        cx.request_paint();

        let cbc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            Size::new(bc.max().width, f64::INFINITY),
        );
        let child_size = self.child.layout(cx, &cbc);
        let size = Size::new(
            child_size.width.min(bc.max().width),
            child_size.height.min(bc.max().height),
        );

        // Ensure that scroll offset is within bounds
        let max_offset = (child_size.height - size.height).max(0.0);
        if max_offset < self.offset {
            self.offset = max_offset;
        }

        size
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        self.child.accessibility(cx);

        if cx.is_requested() {
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::GenericContainer);
            builder.set_children([self.child.id().into()]);
            cx.push_node(builder);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx, scene: &mut Scene) {
        scene.push_layer(Mix::Normal, 1.0, Affine::IDENTITY, &cx.size().to_rect());
        let fragment = self.child.paint_custom(cx);
        scene.append(fragment, Some(Affine::translate((0.0, -self.offset))));
        scene.pop_layer();
    }
}
