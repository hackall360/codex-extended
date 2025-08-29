bitflags::bitflags! {
    #[derive(Default)]
    pub struct Caps: u32 {
        const CHAT          = 1<<0;
        const VISION        = 1<<1;
        const TOOLS         = 1<<2;
        const PARALLEL_TOOLS= 1<<3;
        const JSON_MODE     = 1<<4;
        const JSON_SCHEMA   = 1<<5;
        const SYSTEM_MSG    = 1<<6;
        const STREAMING     = 1<<7;
        const AUDIO_IN      = 1<<8;
        const AUDIO_OUT     = 1<<9;
    }
}
