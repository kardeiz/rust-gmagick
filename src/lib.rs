#[allow(non_upper_case_globals, non_camel_case_types, non_snake_case, unused, improper_ctypes)]
mod ffi;

pub mod err {
    // #[derive(Debug)]
    pub enum Error {
        Gmagick(crate::ExceptionInfo),
        NulError(std::ffi::NulError),
        Io(std::io::Error),
        Boxed(Box<dyn std::error::Error + Send + Sync>)
    }

    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self)
        }
    }

    impl Error {
        pub fn boxed<E: std::error::Error + 'static + Send + Sync>(e: E) -> Self {
            Error::Boxed(Box::new(e))
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use Error::*;
            match self {
                Gmagick(ref e) => {
                    let description = unsafe { std::ffi::CStr::from_ptr(e.0.description) };
                    let reason = unsafe { std::ffi::CStr::from_ptr(e.0.reason) };
                    write!(f, "{}: {}", description.to_string_lossy(), reason.to_string_lossy())?;
                }
                NulError(ref e) => {
                    write!(f, "{}", e)?;
                }
                Io(ref e) => {
                    write!(f, "{}", e)?;
                },
                Boxed(ref e) => {
                    write!(f, "{}", e)?;
                },
            }

            Ok(())
        }
    }

    impl From<std::ffi::NulError> for Error {
        fn from(t: std::ffi::NulError) -> Self { Error::NulError(t) }
    }

    impl std::error::Error for Error {}

    pub type Result<T> = std::result::Result<T, Error>;
}

use std::mem::{self, MaybeUninit};
use std::ptr::{self, NonNull};
use std::ffi::{CStr, CString};
use std::os::raw::c_void;

fn initialize() {
    static INIT: std::sync::Once = std::sync::ONCE_INIT;
    INIT.call_once(|| unsafe {
        ffi::InitializeMagick(ptr::null_mut());
        assert_eq!(libc::atexit(cleanup), 0);
    });

    extern fn cleanup() {
        unsafe { ffi::DestroyMagick(); }
    }
}

pub struct Image(NonNull<ffi::Image>);

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { ffi::DestroyImage(self.0.as_ptr()); }
    }
}

impl Image {

    // pub const RGBA: &'static CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGBA\0") };

    pub fn constitute<T: StorageType>(width: u32, height: u32, map: &CStr, buffer: Vec<T>) -> err::Result<ImageWithContainer<T>> {
        initialize();

        let container = buffer.into_boxed_slice();
        let storage_type = T::STORAGE_TYPE;
        let mut exception_info = ExceptionInfo::new();

        let pixels = container.as_ptr() as *const c_void;

        let ptr = unsafe {
            ffi::ConstituteImage(width as u64, height as u64, map.as_ptr(), storage_type, pixels, &mut exception_info.0)
        };

        match NonNull::new(ptr) {
            Some(ptr) => Ok(ImageWithContainer { image: Image(ptr), container }),
            None => Err(err::Error::Gmagick(exception_info))
        }

    }

    pub fn from_path(path: &str) -> err::Result<ImageWithInfo> {

        initialize();

        unsafe {
            let path_as_c = CString::new(path)?;
            let mut info = ImageInfo::new();
            let mut exception_info = ExceptionInfo::new();

            for (a, &c) in info.0.as_mut().filename
                .iter_mut()
                .zip(path_as_c.as_bytes_with_nul()) {
                *a = c as i8;
            }

            let ptr = ffi::ReadImage(info.0.as_ptr(), &mut exception_info.0);

            match NonNull::new(ptr) {
                Some(ptr) => Ok(ImageWithInfo { image: Image(ptr), info }),
                None => Err(err::Error::Gmagick(exception_info))
            }

        }

    }
}

unsafe impl Send for Image {}
unsafe impl Sync for Image {}


pub struct ImageInfo(NonNull<ffi::ImageInfo>);

impl Drop for ImageInfo {
    fn drop(&mut self) {
        unsafe { ffi::DestroyImageInfo(self.0.as_ptr()); }
    }
}

impl ImageInfo {
    fn new() -> Self {
        let ptr = unsafe { ffi::CloneImageInfo(ptr::null_mut()) };
        ImageInfo(NonNull::new(ptr).expect("Not null per `CloneImageInfo`"))
    }    
}

pub trait StorageType {
    const STORAGE_TYPE: ffi::StorageType;
}

impl StorageType for u8 {
    const STORAGE_TYPE: ffi::StorageType = ffi::StorageType::CharPixel;
}

pub trait ImageLike {
    fn image_ref(&self) -> &Image;
    fn image_mut(&mut self) -> &mut Image;
    fn opt_info_ref(&self) -> Option<&ImageInfo> { None }

    fn name(&self) -> err::Result<String> {
        unsafe {
            let mut vec = Vec::new();
            for bt in self.image_ref().0.as_ref().filename.iter()
                .map(|x| *x as u8)
                .take_while(|x| *x != 0 ) {
                vec.push(bt);
            }
            let out = String::from_utf8(vec).map_err(err::Error::boxed)?;
            Ok(out)
        }
    }

    fn dimensions(&self) -> (u64, u64) {
        unsafe { (self.image_ref().0.as_ref().columns, self.image_ref().0.as_ref().rows) }
    }

    fn mime_type(&self) -> err::Result<String> {
        unsafe {
            let mut vec = Vec::new();
            for bt in self.image_ref().0.as_ref().magick.iter()
                .map(|x| *x as u8)
                .take_while(|x| *x != 0 ) {
                vec.push(bt);
            }
            let out = String::from_utf8(vec).map_err(err::Error::boxed)?;
            Ok(out)
        }
    }

    fn set_format(&mut self, fmt: &str) -> err::Result<()> {
        unsafe {
            let fmt_as_c = CString::new(fmt)?;
            for (a, &c) in self.image_mut().0.as_mut().magick
                .iter_mut()
                .zip(fmt_as_c.as_bytes_with_nul()) {
                *a = c as i8;
            }
            Ok(())
        }
    }

    fn scale(&mut self, w: u64, h: u64) -> err::Result<()> {
        let mut exception_info = ExceptionInfo::new();

        let ptr = unsafe {
            ffi::ScaleImage(
                self.image_ref().0.as_ptr(), 
                w,
                h,
                &mut exception_info.0)
        };

        match NonNull::new(ptr) {
            Some(ptr) => { 
                self.image_mut().0 = ptr;
                Ok(())
            }
            None => Err(err::Error::Gmagick(exception_info))
        }
    }

    fn rotate(&mut self, degrees: f64) -> err::Result<()> {
        let mut exception_info = ExceptionInfo::new();
        let ptr = unsafe {
            ffi::RotateImage(
                self.image_ref().0.as_ptr(),
                degrees,
                &mut exception_info.0)
        };
        match NonNull::new(ptr) {
            Some(ptr) => { 
                self.image_mut().0 = ptr;
                Ok(())
            }
            None => Err(err::Error::Gmagick(exception_info))
        }
    }

    fn crop(
        &mut self,
        x: i64,
        y: i64,
        width: u64, 
        height: u64) -> err::Result<()> {

        let mut exception_info = ExceptionInfo::new();
        let geometry = ffi::RectangleInfo { x, y, width, height };

        let ptr = unsafe { 
            ffi::CropImage(
                self.image_ref().0.as_ptr(),
                &geometry, 
                &mut exception_info.0)
        };

        match NonNull::new(ptr) {
            Some(ptr) => { 
                self.image_mut().0 = ptr;
                Ok(())
            }
            None => Err(err::Error::Gmagick(exception_info))
        }
    }

    fn mirror(&mut self) -> err::Result<()> {
        let mut exception_info = ExceptionInfo::new();
        let ptr = unsafe {
            ffi::FlopImage(self.image_ref().0.as_ptr(), &mut exception_info.0)
        };
        match NonNull::new(ptr) {
            Some(ptr) => { 
                self.image_mut().0 = ptr;
                Ok(())
            }
            None => Err(err::Error::Gmagick(exception_info))
        }
    }

    fn smart_scale(
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

    fn as_bytes(&self) -> err::Result<Vec<u8>> {
        let mut exception_info = ExceptionInfo::new();

        let info;
        let info_ref = match self.opt_info_ref() {
            Some(ref info_ref) => info_ref,
            None => {
                info = ImageInfo::new();
                &info
            }
        };

        unsafe {
            let mut len = usize::default();
            let ptr = ffi::ImageToBlob(
                info_ref.0.as_ptr(), 
                self.image_ref().0.as_ptr(),
                &mut len,
                &mut exception_info.0) as *mut _ as *mut u8;
            
            match NonNull::new(ptr) {
                Some(ptr) => { 
                    let out = std::slice::from_raw_parts(ptr.as_ptr(), len as usize).to_vec();
                    Ok(out)
                }
                None => Err(err::Error::Gmagick(exception_info))
            }            
        }
    }


    fn write(&mut self, path: &str) -> err::Result<()> {
        unsafe {

            let path_as_c = CString::new(path)?;
            
            for (a, &c) in self.image_mut().0.as_mut().filename
                .iter_mut()
                .zip(path_as_c.as_bytes_with_nul()) {
                *a = c as i8;
            }

            let info;
            let info_ref = match self.opt_info_ref() {
                Some(ref info_ref) => info_ref,
                None => {
                    info = ImageInfo::new();
                    &info
                }
            };

            let status = ffi::WriteImage(info_ref.0.as_ptr(), self.image_ref().0.as_ptr());            
            if status == 0 {
                let exception = ExceptionInfo(self.image_ref().0.as_ref().exception.clone());
                Err(err::Error::Gmagick(exception))
            } else {
                Ok(())
            }
        }
    }
}

pub struct ImageWithContainer<T: StorageType> { 
    image: Image, 
    container: Box<[T]>,
}

impl<T: StorageType> ImageLike for ImageWithContainer<T> {
    fn image_ref(&self) -> &Image { &self.image }
    fn image_mut(&mut self) -> &mut Image { &mut self.image }
}

pub struct ImageWithInfo { 
    image: Image, 
    info: ImageInfo
}

impl ImageLike for ImageWithInfo {
    fn image_ref(&self) -> &Image { &self.image }
    fn image_mut(&mut self) -> &mut Image { &mut self.image }
    fn opt_info_ref(&self) -> Option<&ImageInfo> { Some(&self.info) }
}


#[derive(Debug, Clone)]
pub struct ExceptionInfo(ffi::ExceptionInfo);

impl ExceptionInfo {

    pub fn new() -> Self {
        let mut val = MaybeUninit::<ffi::ExceptionInfo>::uninit();
        unsafe { 
            ffi::GetExceptionInfo(val.as_mut_ptr());
            ExceptionInfo(val.assume_init())
        }        
    }

}

impl Drop for ExceptionInfo {
    fn drop(&mut self) {
        unsafe { ffi::DestroyExceptionInfo(&mut self.0); }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {     

        let rgba = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGBA\0") };

        let jp2k::DecodeContainer { buffer, width, height } = jp2k::DecodeContainer::from_file("/mnt/c/projects/jp2k/examples/rust-logo-512x512-blk.jp2", jp2k::Codec::JP2, None)
            .unwrap(); 

        let mut img = Image::constitute(width, height, rgba, buffer).unwrap();

        img.rotate(90.0);

        img.set_format("GIF").unwrap();

        println!("{:?}", img.as_bytes().map_err(|e| { eprintln!("{}", e); e }).unwrap());

        // img.write("rust.png").unwrap();

    }
}
