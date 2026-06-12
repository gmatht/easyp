#[test]
fn print_sizes() {
    eprintln!("HTTP_RESPONSE_V1 size: {}", std::mem::size_of::<lsb_httpapi::HTTP_RESPONSE_V1>());
    eprintln!("HTTP_RESPONSE_HEADERS size: {}", std::mem::size_of::<lsb_httpapi::HTTP_RESPONSE_HEADERS>());
    eprintln!("HTTP_KNOWN_HEADER size: {}", std::mem::size_of::<lsb_httpapi::HTTP_KNOWN_HEADER>());
    eprintln!("HTTP_DATA_CHUNK size: {}", std::mem::size_of::<lsb_httpapi::HTTP_DATA_CHUNK>());
    eprintln!("HTTP_VERSION size: {}", std::mem::size_of::<lsb_httpapi::HTTP_VERSION>());
    eprintln!("HTTP_REQUEST_V1 size: {}", std::mem::size_of::<lsb_httpapi::HTTP_REQUEST_V1>());
    eprintln!("HTTP_COOKED_URL size: {}", std::mem::size_of::<lsb_httpapi::HTTP_COOKED_URL>());
    eprintln!("HTTP_REQUEST_HEADERS size: {}", std::mem::size_of::<lsb_httpapi::HTTP_REQUEST_HEADERS>());
    assert_eq!(std::mem::size_of::<lsb_httpapi::HTTP_RESPONSE_V1>(), 552);
    assert_eq!(std::mem::size_of::<lsb_httpapi::HTTP_RESPONSE_HEADERS>(), 512);
    assert_eq!(std::mem::size_of::<lsb_httpapi::HTTP_REQUEST_V1>(), 1208);
    assert_eq!(std::mem::size_of::<lsb_httpapi::HTTP_COOKED_URL>(), 40);
    assert_eq!(std::mem::size_of::<lsb_httpapi::HTTP_REQUEST_HEADERS>(), 1048);
}
