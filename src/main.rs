// use once_cell::sync::OnceCell;
// static CACHE: OnceCell<Logger> = OnceCell::new();

use anyhow::Result;
use itertools::Itertools;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use iced::{widget::*, *};
use zune_jpeg::zune_core::colorspace::ColorSpace;

pub struct Grid {
    pub columns: usize,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    SetPath,
}

impl Sandbox for Grid {
    type Message = Message;
    fn new() -> Self {
        Grid {
            columns: 3,
            path: None,
        }
    }
    fn title(&self) -> String {
        String::from("Raw Viewer")
    }
    fn update(&mut self, message: Self::Message) {
        match message {
            Message::SetPath => {
                let path = rfd::FileDialog::new().pick_folder();
                self.path = path;
            }
        }
    }
    fn view(&self) -> iced::Element<Self::Message> {
        if let Some(ref path) = self.path {
            widget::container(widget::column(
                std::fs::read_dir(path)
                    .unwrap()
                    .flatten()
                    .flat_map(|p| view_from_raw(p.path()))
                    .map(|p| p.width(Length::FillPortion(1)))
                    .map(Into::into)
                    .chunks(self.columns)
                    .into_iter()
                    .map(|c| widget::row(c.collect::<Vec<_>>()).width(Length::Fill))
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            ))
        } else {
            // Align to center
            widget::container(widget::button("Open").on_press(Message::SetPath))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
        }
        .into()
    }
}

fn main() -> Result<()> {
    <Grid as Sandbox>::run(Settings::default()).unwrap();
    Ok(())
}

pub fn view_from_raw(path: impl AsRef<Path>) -> Result<image::Viewer<image::Handle>> {
    let skip = 1;
    use libraw_r::Processor;
    let mut proc = Processor::default();
    proc.open(&path)?;

    let list = proc.thumbs_list();
    //    dbg!(&list);
    let list: Vec<_> = list.thumblist[..list.thumbcount as usize]
        .iter()
        .filter(|v| v.tformat == 4)
        // .filter(|v| v.tlength > 2u32.pow(14))
        .sorted_by_key(|v| v.tlength)
        .collect();

    let item = if list.len() > skip as usize + 1 {
        list[skip as usize]
    } else if !list.is_empty() {
        list.last().unwrap()
    } else {
        return Err(anyhow::anyhow!("No thumbnails found"));
        // return Ok(proc.to_jpeg_no_rotation(75)?);
    };
    let mut f = std::fs::File::open(&path)?;
    f.seek(SeekFrom::Start(item.toffset as u64))?;
    let mut buf = Vec::with_capacity(item.tlength as usize + 1);
    f.take(item.tlength as u64 + 1).read_to_end(&mut buf)?;

    let mut decoder = JpegDecoder::new(&buf);
    decoder.set_options(DecoderOptions::new_fast().jpeg_set_out_colorspace(ColorSpace::RGBA));
    let pixels = decoder.decode()?;
    let (width, height) = decoder.dimensions().unwrap();
    Ok(image::viewer(image::Handle::from_pixels(
        width.into(),
        height.into(),
        pixels,
    )))
}
