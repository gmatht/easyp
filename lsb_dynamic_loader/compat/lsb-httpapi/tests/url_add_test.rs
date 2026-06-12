#[repr(C)]
#[derive(Clone, Copy)]
struct HTTPAPI_VERSION {
    http_major: u16,
    http_minor: u16,
}

type ULONG = u32;

#[link(name = "httpapi")]
extern "system" {
    fn HttpInitialize(version: HTTPAPI_VERSION, flags: ULONG, reserved: *mut std::ffi::c_void) -> ULONG;
    fn HttpCreateServerSession(version: HTTPAPI_VERSION, id: *mut u64, reserved: ULONG) -> ULONG;
    fn HttpCreateUrlGroup(session: u64, group: *mut u64, reserved: ULONG) -> ULONG;
    fn HttpAddUrlToUrlGroup(group: u64, url: *const u16, context: u64, reserved: ULONG) -> ULONG;
    fn HttpRemoveUrlFromUrlGroup(group: u64, url: *const u16, flags: ULONG) -> ULONG;
    fn HttpCloseUrlGroup(group: u64) -> ULONG;
    fn HttpCloseServerSession(session: u64) -> ULONG;
    fn HttpTerminate(flags: ULONG, reserved: *mut std::ffi::c_void) -> ULONG;
}

const HTTP_INITIALIZE_SERVER: ULONG = 1;

fn main() {
    let ver = HTTPAPI_VERSION { http_major: 2, http_minor: 0 };
    let mut session = 0u64;
    let mut group = 0u64;

    let url1 = "http://+:18888/";
    let url2 = "http://+:21999/";
    let wide1: Vec<u16> = url1.encode_utf16().chain(Some(0)).collect();
    let wide2: Vec<u16> = url2.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let rc = HttpInitialize(ver, HTTP_INITIALIZE_SERVER, std::ptr::null_mut());
        eprintln!("HttpInitialize = {}", rc);

        let rc = HttpCreateServerSession(ver, &mut session, 0);
        eprintln!("HttpCreateServerSession = {} id={:#x}", rc, session);

        let rc = HttpCreateUrlGroup(session, &mut group, 0);
        eprintln!("HttpCreateUrlGroup = {} id={:#x}", rc, group);

        // Try url2 FIRST
        let rc = HttpAddUrlToUrlGroup(group, wide2.as_ptr(), 0_u64, 0);
        eprintln!("HttpAddUrlToUrlGroup({}) = {}", url2, rc);

        // Now try url1
        let rc = HttpAddUrlToUrlGroup(group, wide1.as_ptr(), 0_u64, 0);
        eprintln!("HttpAddUrlToUrlGroup({}) = {}", url1, rc);

        HttpRemoveUrlFromUrlGroup(group, wide1.as_ptr(), 0);
        HttpRemoveUrlFromUrlGroup(group, wide2.as_ptr(), 0);
        HttpCloseUrlGroup(group);
        HttpCloseServerSession(session);
        HttpTerminate(HTTP_INITIALIZE_SERVER, std::ptr::null_mut());
    }
}
