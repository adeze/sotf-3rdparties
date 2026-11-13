mod base_absolute {
    #[vst3_com::com_interface("12345678-1234-1234-1234-12345678ABCD")]
    pub trait IBaseAbsolute: vst3_com::interfaces::iunknown::IUnknown {}
}

mod specific_absolute {
    #[vst3_com::com_interface("12345678-1234-1234-1234-12345678ABCE")]
    trait ISpecificAbsolute: crate::base_absolute::IBaseAbsolute {}
}

mod specific_relative {
    #[vst3_com::com_interface("12345678-1234-1234-1234-12345678ABCE")]
    trait ISpecificRelative: super::base_absolute::IBaseAbsolute {}
}

mod base_use {
    use vst3_com::interfaces::iunknown::IUnknown;

    #[vst3_com::com_interface("12345678-1234-1234-1234-12345678ABCD")]
    pub trait IBaseUse: IUnknown {}
}

mod specific_use {
    use crate::base_use::IBaseUse;

    #[vst3_com::com_interface("12345678-1234-1234-1234-12345678ABCE")]
    trait ISpecificUse: IBaseUse {}
}

fn main() {}
