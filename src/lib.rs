#![allow(dead_code, non_camel_case_types, non_upper_case_globals, non_snake_case)]

extern crate libc;

use std::ptr;
use std::mem;
use std::ffi::CString;

mod ffi;

pub struct Image {
  pub ptr: *mut ffi::Image
}

impl Drop for Image {
  fn drop(&mut self) {
    if !self.ptr.is_null() { 
      unsafe { ffi::DestroyImage(self.ptr); }
    }
  }
}

impl Image {

  pub fn new() -> Image {
    Image { ptr: ptr::null_mut() }
  }

  fn from_ptr(ptr: *mut ffi::Image) -> Option<Image> {
    if ptr.is_null() { 
      None
    } else { 
      Some(Image { ptr: ptr })
    }
  }

  pub fn from_path(_path: &str) -> Option<Image> {
    let path = CString::new(_path).unwrap();
    let img_info  = ImageInfo::new();
    let mut exception = ExceptionInfo::new();
    
    let ptr = unsafe {             
      ffi::InitializeMagick(ptr::null_mut());
      for (a, &c) in (*img_info.ptr).filename
        .iter_mut()
        .zip(path.as_bytes_with_nul()) {
        *a = c as i8;
      }
      ffi::ReadImage(img_info.ptr, &mut exception.val)
    };
    Self::from_ptr(ptr)    
  }

  pub fn dimensions(&mut self) -> (u64, u64) {
    unsafe { ((*self.ptr).columns, (*self.ptr).rows) }
  }

  pub fn scale(&mut self, w: u64, h: u64) -> Option<Image> {
    let mut exception = ExceptionInfo::new();
    let ptr = unsafe {
      ffi::ScaleImage(self.ptr, 
        w as ::libc::c_ulong,
        h as ::libc::c_ulong,
        &mut exception.val)
    };
    Self::from_ptr(ptr)
  }

  pub fn rotate(&mut self, degrees: f64) -> Option<Image> {
    let mut exception = ExceptionInfo::new();
    let ptr = unsafe {
      ffi::RotateImage(self.ptr, 
        degrees as ::libc::c_double,
        &mut exception.val)
    };
    Self::from_ptr(ptr)
  }

  pub fn mirror(&mut self) -> Option<Image> {
    let mut exception = ExceptionInfo::new();
    let ptr = unsafe {
      ffi::FlopImage(self.ptr, &mut exception.val)
    };
    Self::from_ptr(ptr)
  }

  pub fn crop(&mut self,
    x: i64,
    y: i64,
    width: u64, 
    height: u64) -> Option<Image> {

    let mut exception = ExceptionInfo::new();

    let geometry = ffi::RectangleInfo {
      x: x as ::libc::c_long,
      y: y as ::libc::c_long,
      width: width as ::libc::c_ulong,
      height: height as ::libc::c_ulong
    };

    let ptr = unsafe { 
      ffi::CropImage(self.ptr, &geometry, &mut exception.val)
    };

    Self::from_ptr(ptr)

  }

  pub fn write(&mut self, _path: &str) -> Result<(), ()> {
    let path = CString::new(_path).unwrap();
    let img_info  = ImageInfo::new();
    let res = unsafe {
      for (a, &c) in (*self.ptr).filename
        .iter_mut()
        .zip(path.as_bytes_with_nul()) {
        *a = c as i8;
      }
      ffi::WriteImage(img_info.ptr, self.ptr)
    };
    if res == 0 {
      Err(())
    } else {
      Ok(())
    }
  }

  pub fn smart_scale(&mut self, 
    _w: Option<u64>,
    _h: Option<u64>) -> Option<Image> {

    let (cw, ch) = self.dimensions();

    let ratio_w = _w
      .map(|w| (w as f64) / (cw as f64))
      .unwrap_or(1 as f64);

    let ratio_h = _h
      .map(|h| (h as f64) / (ch as f64))
      .unwrap_or(1 as f64);

    let ratio = if ratio_w < ratio_h { 
      ratio_w
    } else { 
      ratio_h
    };

    let w = ((cw as f64) * ratio).ceil() as u64;
    let h = ((ch as f64) * ratio).ceil() as u64;

    self.scale(w, h)
  }
}

// ImageInfo

pub struct ImageInfo {
  pub ptr: *mut ffi::ImageInfo
}

impl Drop for ImageInfo {
  fn drop(&mut self) {
    if !self.ptr.is_null() { 
      unsafe { ffi::DestroyImageInfo(self.ptr); }
    }
  }
}

impl ImageInfo {

  pub fn new() -> ImageInfo {
    let ptr = 
      unsafe { ffi::CloneImageInfo(ptr::null_mut() as *const ffi::ImageInfo) };
    ImageInfo { ptr: ptr }
  }

}


pub struct ExceptionInfo {
  pub val: ffi::ExceptionInfo
}

impl Drop for ExceptionInfo {
  fn drop(&mut self) {
    unsafe { ffi::DestroyExceptionInfo(&mut self.val); }
  }
}

impl ExceptionInfo {

  pub fn new() -> ExceptionInfo {
    let mut val: ffi::ExceptionInfo = 
      unsafe { mem::uninitialized() };
    unsafe { ffi::GetExceptionInfo (&mut val); };
    ExceptionInfo { val: val }
  }

}
