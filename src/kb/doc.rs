// use std::fs::File;
// use std::io::Read;
// use std::path::Path;
use std::vec::Vec;

use docx_rs::read_docx;

use crate::result::Result;

pub(super) fn parse_docx(buf: Vec<u8>) -> Result<String> {
    // let mut file = File::open("./numbering.docx")?;
    // let mut buf = Vec::with_capacity(3096);
    // file.read_to_end(&mut buf)?;
    let mut doc_text = String::with_capacity(3096);
    let docx = read_docx(&buf)?;
    let doc = docx.document;
    for d in doc.children.iter() {
        match d {
            docx_rs::DocumentChild::Paragraph(paragraph) => {
                for p in paragraph.children() {
                    match p {
                        docx_rs::ParagraphChild::Run(run) => {
                            for r in run.children.iter() {
                                match r {
                                    docx_rs::RunChild::Text(text) => {
                                        // log::info!("Docx text={}", text.text);
                                        doc_text.push_str(&text.text);
                                        doc_text.push('\n');
                                        doc_text.push('\n');
                                    }
                                    docx_rs::RunChild::Sym(sym) => {
                                        doc_text.push_str(&sym.char);
                                    }
                                    docx_rs::RunChild::Break(_) => {
                                        doc_text.push('\n');
                                    }
                                    _ => {}
                                }
                            }
                        }
                        docx_rs::ParagraphChild::Hyperlink(hyperlink) => {
                            log::info!("hyperlink: {:?}", hyperlink.link)
                        }
                        _ => {}
                    }
                }
            }
            docx_rs::DocumentChild::Table(_table) => {}
            docx_rs::DocumentChild::TableOfContents(_table_of_contents) => {}
            _ => {}
        }
    }
    Ok(doc_text)
}

fn parse_pdf() {}
