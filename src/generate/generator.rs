use std::path::Path;

use super::Writer;

#[allow(dead_code, unused_variables)]
pub fn generate<P: AsRef<Path>, W: Writer, G: Generator<W>>(
    out_dir: P,
    registry: Vec<()>,
    generator: &G,
) -> anyhow::Result<()> {
    todo!()
}

#[allow(dead_code)]
pub trait Generator<W>
where
    W: Writer,
{
    // fn package(writer: &mut W, )
    // fn message(writer: &mut W, value: &Message);
    // fn enumeration(writer: &mut W, value: &Enum);
}
