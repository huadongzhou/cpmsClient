// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use gdk::{Display, Rectangle, Screen};

use crate::{
  dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
  monitor::{MonitorHandle as RootMonitorHandle, VideoMode as RootVideoMode},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonitorHandle {
  screen: Screen,
  pub(crate) number: i32,
}

impl MonitorHandle {
  #[allow(deprecated)]
  pub fn new(display: &gdk::Display, number: i32) -> Option<Self> {
    let screen = display.default_screen();
    if number >= 0 && number < screen.n_monitors() {
      Some(Self { screen, number })
    } else {
      None
    }
  }

  #[inline]
  #[allow(deprecated)]
  pub fn name(&self) -> Option<String> {
    self
      .screen
      .monitor_plug_name(self.number)
      .map(|s| s.as_str().to_string())
  }

  #[inline]
  pub fn size(&self) -> PhysicalSize<u32> {
    let rect = self.geometry();
    LogicalSize {
      width: rect.width() as u32,
      height: rect.height() as u32,
    }
    .to_physical(self.scale_factor())
  }

  #[inline]
  pub fn position(&self) -> PhysicalPosition<i32> {
    let rect = self.geometry();
    LogicalPosition {
      x: rect.x(),
      y: rect.y(),
    }
    .to_physical(self.scale_factor())
  }

  #[inline]
  #[allow(deprecated)]
  pub fn scale_factor(&self) -> f64 {
    self.screen.monitor_scale_factor(self.number) as f64
  }

  #[inline]
  pub fn video_modes(&self) -> Box<dyn Iterator<Item = RootVideoMode>> {
    Box::new(Vec::new().into_iter())
  }

  #[inline]
  #[allow(deprecated)]
  fn geometry(&self) -> Rectangle {
    self.screen.monitor_geometry(self.number)
  }
}

unsafe impl Send for MonitorHandle {}
unsafe impl Sync for MonitorHandle {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoMode;

impl VideoMode {
  #[inline]
  pub fn size(&self) -> PhysicalSize<u32> {
    panic!("VideoMode is unsupported on Linux.")
  }

  #[inline]
  pub fn bit_depth(&self) -> u16 {
    panic!("VideoMode is unsupported on Linux.")
  }

  #[inline]
  pub fn refresh_rate(&self) -> u16 {
    panic!("VideoMode is unsupported on Linux.")
  }

  #[inline]
  pub fn monitor(&self) -> RootMonitorHandle {
    panic!("VideoMode is unsupported on Linux.")
  }
}

pub fn from_point(display: &Display, x: f64, y: f64) -> Option<MonitorHandle> {
  let screen = display.default_screen();
  #[allow(deprecated)]
  let number = screen.monitor_at_point(x as i32, y as i32);
  MonitorHandle::new(display, number)
}
