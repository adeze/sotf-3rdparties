use runtime::Imp;

extern "C" {
    fn objc_msgSend();

    fn objc_msgSendSuper();
}

#[expect(clippy::extra_unused_type_parameters, reason = "Keep the shared dispatcher interface; arm64 uses one entry point for every return type")]
pub fn msg_send_fn<R>() -> Imp {
    // stret is not even available in arm64.
    // <https://twitter.com/gparker/status/378079715824660480>

    objc_msgSend
}

#[expect(clippy::extra_unused_type_parameters, reason = "Keep the shared dispatcher interface; arm64 uses one entry point for every return type")]
pub fn msg_send_super_fn<R>() -> Imp {
    objc_msgSendSuper
}
