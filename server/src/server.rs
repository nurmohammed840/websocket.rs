use super::*;

pub struct Data<'a> {
    pub(crate) len: usize,
    pub(crate) ty: DataType,

    pub(crate) ws: &'a mut Websocket<SERVER>,
}
