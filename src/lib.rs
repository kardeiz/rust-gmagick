#![allow(dead_code, non_camel_case_types, non_upper_case_globals, non_snake_case)]

extern crate libc;

extern crate failure;

#[macro_use]
extern crate failure_derive;

use std::ptr;
use std::mem;
use std::ffi::{CStr, CString};

use std::sync::{Once, ONCE_INIT};

use std::os::raw;

mod ffi;

pub mod err {

    #[derive(Fail, Debug)]
    pub enum Error {
        #[fail(display = "{}", _0)]
        FromUtf8(#[cause] ::std::string::FromUtf8Error),
        #[fail(display = "{}", _0)]
        Utf8(#[cause] ::std::str::Utf8Error),
        #[fail(display = "{}", _0)]
        Nul(#[cause] ::std::ffi::NulError),
        #[fail(display = "Something went wrong")]
        Other
    }

    impl From<::std::string::FromUtf8Error> for Error {
        fn from(t: ::std::string::FromUtf8Error) -> Self { Error::FromUtf8(t) }
    }

    impl From<::std::str::Utf8Error> for Error {
        fn from(t: ::std::str::Utf8Error) -> Self { Error::Utf8(t) }
    }

    impl From<::std::ffi::NulError> for Error {
        fn from(t: ::std::ffi::NulError) -> Self { Error::Nul(t) }
    }

    pub type Result<T> = ::std::result::Result<T, Error>;

}

fn initialize() {
    static INIT: Once = ONCE_INIT;
    INIT.call_once(|| unsafe {
        ffi::InitializeMagick(ptr::null_mut());
        assert_eq!(::libc::atexit(cleanup), 0);
    });

    extern fn cleanup() {
        unsafe { ffi::DestroyMagick(); }
    }
}

pub struct Worker {
    pub image: Image,
    pub info: ImageInfo,
    pub exception: ExceptionInfo
}

impl ::std::default::Default for Worker {
    fn default() -> Self {
        Worker {
            image: Image { ptr: ptr::null_mut() },
            info: ImageInfo::new(),
            exception: ExceptionInfo::new()
        }
    }
}

impl Worker {

    pub fn new() -> Self {
        initialize();
        Worker::default()
    }

    pub fn name(&self) -> err::Result<String> {
        unsafe {
            let mut vec = Vec::new();
            for bt in (*self.image.ptr).filename.iter()
                .map(|x| *x as u8)
                .take_while(|x| *x != 0 ) {
                vec.push(bt);
            }
            let out = String::from_utf8(vec)?;
            Ok(out)
        }
    }

    pub fn mime_type(&self) -> err::Result<String> {
        unsafe {
            let mime = ffi::MagickToMime(&(*self.image.ptr).magick as *const i8);
            let out = CStr::from_ptr(mime).to_str()?.to_owned();
            Ok(out)
        }
    }

    pub fn from_path(path: &str) -> err::Result<Self> {
        unsafe {
            let mut worker = Worker::new();
            let path_c = CString::new(path)?;
            let info = worker.info.clone();
            for (a, &c) in (*info.ptr).filename
                .iter_mut()
                .zip(path_c.as_bytes_with_nul()) {
                *a = c as i8;
            }

            let ptr = ffi::ReadImage(info.ptr, &mut worker.exception.val);

            worker.image = Image::from_ptr(ptr)?;

            let _ = worker.cache()?;

            Ok(worker)
        }
    }

    pub fn from_cache(path: &str) -> err::Result<Self> {
        unsafe {
            let mut worker = Worker::new();
            let mut id = i64::default();
            let path_c = CString::new(path)?;
            let info = worker.info.clone();
            for (a, &c) in (*info.ptr).filename
                .iter_mut()
                .zip(path_c.as_bytes_with_nul()) {
                *a = c as i8;
            }

            let ptr = ffi::GetImageFromMagickRegistry(
                &(*info.ptr).filename as *const i8,
                &mut id,
                &mut worker.exception.val);
            worker.image = Image::from_ptr(ptr)?;
            Ok(worker)
        }
    }

    pub fn write(&mut self, path: &str) -> err::Result<()> {
        unsafe {
            let path = CString::new(path)?;
            let info = self.info.clone();
            
            for (a, &c) in (*self.image.ptr).filename
                .iter_mut()
                .zip(path.as_bytes_with_nul()) {
                *a = c as i8;
            }
            let status = ffi::WriteImage(info.ptr, self.image.ptr);
            
            if status == 0 {
                Err(err::Error::Other)
            } else {
                Ok(())
            }
        }
    }

    pub fn set_format(&mut self, fmt: &str) -> err::Result<()> {
        unsafe {
            let fmt_c = CString::new(fmt)?;
            for (a, &c) in (*self.image.ptr).magick
                .iter_mut()
                .zip(fmt_c.as_bytes_with_nul()) {
                *a = c as i8;
            }
            Ok(())
        }
    }

    pub fn set_quality(&mut self, quality: u64) -> err::Result<()> {
        unsafe {
            (*self.info.ptr).quality = quality;
            Ok(())
        }
    }

    pub fn cache(&mut self) -> err::Result<i64> {
        unsafe {
            let mut exception = self.exception.clone();
            let id = ffi::SetMagickRegistry(
                ffi::RegistryType::ImageRegistryType,
                self.image.ptr as *mut _ as *mut raw::c_void,
                mem::size_of::<ffi::Image>(),
                &mut exception.val);
            if id == -1 {
                Err(err::Error::Other)
            } else {
                Ok(id)
            }
        }
    }

    pub fn write_bytes(&self) -> err::Result<Vec<u8>> {
        unsafe {
            let mut exception = self.exception.clone();
            let mut len = usize::default();
            let ptr = ffi::ImageToBlob(
                self.info.ptr, 
                self.image.ptr,
                &mut len,
                &mut exception.val) as *mut _ as *mut u8;
            if ptr.is_null() {
                Err(err::Error::Other)
            } else {
                let out = ::std::slice::from_raw_parts(ptr, len as usize).to_vec();
                Ok(out)
            }
            
        }
    }

    pub fn get(path: &str) -> err::Result<Self> {
        Self::from_cache(path)
            .or_else(|_| Self::from_path(path))
    }

    pub fn dimensions(&self) -> (u64, u64) {
        unsafe { ((*self.image.ptr).columns, (*self.image.ptr).rows) }
    }

    pub fn scale(&mut self, w: u64, h: u64) -> err::Result<()> {
        let ptr = unsafe {
            ffi::ScaleImage(
                self.image.ptr, 
                w as raw::c_ulong,
                h as raw::c_ulong,
                &mut self.exception.val)
        };
        self.image = Image::from_ptr(ptr)?;
        Ok(())
    }

    pub fn rotate(&mut self, degrees: f64) -> err::Result<()> {
        let ptr = unsafe {
            ffi::RotateImage(
                self.image.ptr, 
                degrees as raw::c_double,
                &mut self.exception.val)
        };
        self.image = Image::from_ptr(ptr)?;
        Ok(())
    }

    pub fn mirror(&mut self) -> err::Result<()> {
        let ptr = unsafe {
            ffi::FlopImage(self.image.ptr, &mut self.exception.val)
        };
        self.image = Image::from_ptr(ptr)?;
        Ok(())
    }

    pub fn crop(
        &mut self,
        x: i64,
        y: i64,
        width: u64, 
        height: u64) -> err::Result<()> {

        let geometry = ffi::RectangleInfo {
            x: x as raw::c_long,
            y: y as raw::c_long,
            width: width as raw::c_ulong,
            height: height as raw::c_ulong
        };

        let ptr = unsafe { 
            ffi::CropImage(
                self.image.ptr, 
                &geometry, 
                &mut self.exception.val)
        };

        self.image = Image::from_ptr(ptr)?;
        Ok(())
    }

    pub fn smart_scale(
        &mut self, 
        w: Option<u64>,
        h: Option<u64>) -> err::Result<()> {

        let (cw, ch) = self.dimensions();

        let ratio_w = w
            .map(|w| (w as f64) / (cw as f64))
            .unwrap_or(1 as f64);

        let ratio_h = h
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

pub struct Image {
    pub ptr: *mut ffi::Image
}

impl Image {
    fn from_ptr(ptr: *mut ffi::Image) -> err::Result<Image> {
        if ptr.is_null() { 
            Err(err::Error::Other)
        } else { 
            Ok(Image { ptr: ptr })
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if !self.ptr.is_null() { 
            unsafe { ffi::DestroyImage(self.ptr); }
        }
    }
}

pub struct ImageInfo {
    pub ptr: *mut ffi::ImageInfo
}

impl Clone for ImageInfo {

    fn clone(&self) -> Self {
        ImageInfo { ptr: unsafe { ffi::CloneImageInfo(self.ptr) } }
    }

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
        let ptr = unsafe { ffi::CloneImageInfo(ptr::null_mut()) };
        ImageInfo { ptr: ptr }
    }

}

#[derive(Clone)]
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
        let mut val: ffi::ExceptionInfo = unsafe { mem::uninitialized() };
        unsafe { ffi::GetExceptionInfo(&mut val); }
        ExceptionInfo { val: val }
    }

}
