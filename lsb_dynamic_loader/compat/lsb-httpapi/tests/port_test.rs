#[cfg(windows)]
mod test {
    #[test]
    fn test_port_9776() {
        use std::os::windows::ffi::OsStrExt;

        #[repr(C)]
        #[derive(Clone, Copy)]
        struct HTTPAPI_VERSION { http_major: u16, http_minor: u16 }
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
        let ver = HTTPAPI_VERSION { http_major: 2, http_minor: 0 };
        let mut session = 0u64;
        let mut group = 0u64;

        unsafe {
            let rc = HttpInitialize(ver, HTTP_INITIALIZE_SERVER, std::ptr::null_mut());
            assert_eq!(rc, 0, "HttpInitialize failed: {}", rc);

            let rc = HttpCreateServerSession(ver, &mut session, 0);
            assert_eq!(rc, 0, "HttpCreateServerSession failed: {}", rc);

            let rc = HttpCreateUrlGroup(session, &mut group, 0);
            assert_eq!(rc, 0, "HttpCreateUrlGroup failed: {}", rc);

            // Test port 9776
            let url = "http://+:9776/";
            let wide: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
            let rc = HttpAddUrlToUrlGroup(group, wide.as_ptr(), 0_u64, 0);
            eprintln!("HttpAddUrlToUrlGroup({}) = {}", url, rc);
            assert_eq!(rc, 0, "HttpAddUrlToUrlGroup({}) failed: {}", url, rc);

            // Test port 9777
            let url2 = "https://+:9777/";
            let wide2: Vec<u16> = url2.encode_utf16().chain(Some(0)).collect();
            let rc2 = HttpAddUrlToUrlGroup(group, wide2.as_ptr(), 0_u64, 0);
            eprintln!("HttpAddUrlToUrlGroup({}) = {}", url2, rc2);
            assert_eq!(rc2, 0, "HttpAddUrlToUrlGroup({}) failed: {}", url2, rc2);

            // Cleanup
            HttpRemoveUrlFromUrlGroup(group, wide.as_ptr(), 0);
            HttpRemoveUrlFromUrlGroup(group, wide2.as_ptr(), 0);
            HttpCloseUrlGroup(group);
            HttpCloseServerSession(session);
            HttpTerminate(HTTP_INITIALIZE_SERVER, std::ptr::null_mut());
        }
    }
}
