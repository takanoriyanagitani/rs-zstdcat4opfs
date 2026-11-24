use ruzstd::io_nostd::Error;
use ruzstd::io_nostd::ErrorKind;
use ruzstd::io_nostd::Read;
use ruzstd::io_nostd::Write;

use ruzstd::decoding::StreamingDecoder;

#[link(wasm_import_module = "env")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn opfs_read(ptr: *mut u8, len: u32, offset: u32) -> i32;

    fn bytes_sink(ptr: *const u8, len: u32) -> i32;
}

#[derive(Default)]
pub struct BytesSink;

impl Write for BytesSink {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let ptr: *const u8 = buf.as_ptr();
        let siz: u32 = buf.len() as u32;
        #[allow(unsafe_code)]
        let n: i32 = unsafe { bytes_sink(ptr, siz) };
        if n < 0 {
            return Err(Error::new(ErrorKind::Other, Box::new("bytes sink error")));
        }
        Ok(n as usize)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct OpfsRead {
    position: usize,
}

impl Read for OpfsRead {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let ptr: *mut u8 = buf.as_mut_ptr();
        let siz: u32 = buf.len() as u32;
        let offset: u32 = self.position as u32;
        #[allow(unsafe_code)]
        let n: i32 = unsafe { opfs_read(ptr, siz, offset) };
        if n < 0 {
            return Err(Error::new(ErrorKind::Other, Box::new("opfs read error")));
        }
        let bytes_read: usize = n as usize;
        self.position += bytes_read;
        Ok(bytes_read)
    }
}

pub fn io_rdr2zstd2writer<R, W>(rdr: &mut R, wtr: &mut W, max: u64) -> Result<u64, &'static str>
where
    R: Read,
    W: Write,
{
    let dec = StreamingDecoder::new(rdr).map_err(|_| "unable to create StreamingDecoder")?;
    let mut taken = dec.take(max);
    let mut buf: Vec<u8> = vec![0; 65536];
    let mut tot: u64 = 0;
    loop {
        let cnt: usize = taken.read(&mut buf).map_err(|_| "read error")?;
        if 0 == cnt {
            return Ok(tot);
        }

        tot = tot.saturating_add(cnt as u64);

        let s: &[u8] = &buf[0..cnt];
        wtr.write_all(s).map_err(|_| "write error")?;
    }
}

pub fn io_opfs2zstd2wtr<W>(wtr: &mut W, max: u64) -> Result<u64, &'static str>
where
    W: Write,
{
    let mut rdr = OpfsRead::default();
    io_rdr2zstd2writer(&mut rdr, wtr, max)
}

pub fn io_opfs2zstd2sink(max: u64) -> Result<u64, &'static str> {
    let mut wtr = BytesSink;
    io_opfs2zstd2wtr(&mut wtr, max)
}

pub fn io_opfs2zcat2sink(max: u32) -> Result<u32, &'static str> {
    io_opfs2zstd2sink(max.into()).map(|u| u as u32)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn e_io_opfs2zcat2sink(max: u32) -> i32 {
    io_opfs2zcat2sink(max).map(|u| u as i32).unwrap_or(-1)
}
