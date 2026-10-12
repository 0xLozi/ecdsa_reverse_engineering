pub struct ECDSA_POINT <'steve> {
    pub x: &'steve mut [u8;0x14],
    pub y: &'steve mut [u8;0x14],
} // 0x28

impl <'steve> ECDSA_POINT <'steve> {
    pub fn new(outbuff: &'steve mut [u8]) -> Self {

        let (x_slice, rest_buffer) = outbuff.split_at_mut(0x14);
        let (y_slice, _) = rest_buffer.split_at_mut(0x14);

        Self {
            x: x_slice.try_into().unwrap(),
            y: y_slice.try_into().unwrap()
        }
    }
}

pub struct Kirk_CMD12_BUFFER <'axel>{
    pub private_key: &'axel mut [u8;0x14],
    pub public_key: ECDSA_POINT<'axel>,
}

impl <'axel> Kirk_CMD12_BUFFER <'axel> {
    
    pub fn new(outbuff: &'axel mut [u8]) -> Self {
        let (private_key, rest_buffer) = outbuff.split_at_mut(0x14);
        let public_key = ECDSA_POINT::new(&mut rest_buffer[..0x28]);

        Self {
            private_key: private_key.try_into().unwrap(),
            public_key
        }
    }
}
