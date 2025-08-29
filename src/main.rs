use log::*;

extern crate imgui;


extern crate pdf;

pub mod search;
use search::*;

use imgui::*;
use std::str;

use pdf::*;
use object::*;
use backend::*;
use primitive::*;

use std::cell::RefCell;
use std::fs;
use std::env;



mod support;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        let file_name = "files/example.pdf";
        main_withfile(file_name.to_owned());
    }
}

fn main() {
    let file_name = env::args().nth(1).unwrap_or("files/pdf-sample.pdf".to_string() );
    main_withfile(file_name);
}
fn main_withfile(file_name: String) {
    stderrlog::new().module(module_path!()).verbosity(2).init().unwrap();

    let data = fs::read(file_name).expect("could not open file");
    let (xref_tab, trailer) = data.read_xref_table_and_trailer().unwrap();
    let storage = pdf::file::Storage::new(data, xref_tab);

    let search_paths = RefCell::new(Vec::new());


    support::simple_init(file!(), move  |_, ui| {
        let inspector = Inspector::new(ui, &storage);
        ui.window("Inspect PDF")
            .size([800f32, 600f32], imgui::Condition::FirstUseEver)
            .position([100f32, 110f32], imgui::Condition::FirstUseEver)
            .build(|| {
                inspector.draw(ui, &trailer);
            });

        // Search window
        ui.window("Search PDF")
            .size([300f32, 100f32], imgui::Condition::FirstUseEver)
            .position([100f32, 0f32], imgui::Condition::FirstUseEver)
            .build(|| {
                let mut search_term = String::with_capacity(20);
                if ui.input_text("Search term", &mut search_term).enter_returns_true(true).build() {
                    // Start search!
                    log::info!("Searching for key '{}'", search_term);
                    let search_paths = &mut *search_paths.borrow_mut();
                    *search_paths = inspector.search_key(&trailer.clone().into(), &search_term);

                    log::info!("Paths: {:?}", search_paths);
                }
                for path in &*search_paths.borrow() {
                    ui.text(format!("{}", path_to_string(&path)));
                }
            });

        
    });
}

fn path_to_string(path: &SearchPath) -> String {
    let mut result: String = String::new();
    for elem in path.iter() {
        match *elem {
            PathElem::DictElem {ref key} => {
                result += "->";
                result += &key
            }
            PathElem::ArrayElem {index} => {
                result += &format!("[{}]", index);
            }
        }
    }
    result
}


struct Inspector<'a,  R: Resolve> {
    ui: &'a Ui,
    resolve: &'a R ,
}

impl<'a,  R: Resolve> Inspector<'a,  R> {
    pub fn new(ui: &'a Ui, resolve: &'a R) -> Inspector<'a,  R> {
        Inspector {
            ui: ui,
            resolve: resolve,
        }
    }
    pub fn draw(&self, ui: &Ui, root: &Dictionary) {
        ui.text("PDF file");
        ui.separator();
        self.view_dict(root);
    }

    pub fn view_primitive(&self, prim: &Primitive) {
        match *prim {
            Primitive::Null => {log::debug!("Null"); self.ui.text("null");},
            Primitive::Integer (x) => {log::debug!("Integer: {}", x); self.ui.text(format!("{}", x));},
            Primitive::Number (x) => {log::debug!("Number: {}", x); self.ui.text(format!("{}", x));},
            Primitive::Boolean (x) => {log::debug!("Boolean: {}", x); self.ui.text(format!("{}", x));},
            Primitive::String (ref x) => {
                log::debug!("String: {:?}", x); 
                self.ui.text(format!("\"{}\"", x.as_str().unwrap_or("<indiscernible string>".into())));
            },
            Primitive::Stream (ref x) => {
                log::debug!("Stream");
                self.attr("Data", &PdfString::new(x.data.clone()).into(), 0);
                self.attr("Info", &x.info.clone().into(), 1);
                self.ui.tree_node("Info").map(|_| self.view_dict(&x.info));
            }
            Primitive::Dictionary (ref x) => {log::debug!("Dictionary"); self.view_dict(x)},
            Primitive::Array (ref x) => {
                log::debug!("Array of length {}", x.len());
                for (i, prim) in x.iter().enumerate() {
                    let i = i as i32;
                    self.attr(&format!("elem{}", i), prim, i);
                }
            }
            Primitive::Reference (ref x) => {
                log::debug!("Reference");
                match self.resolve.resolve(*x) {
                    Ok(primitive) => {
                        self.attr(&format!("Ref[{}, {}]", x.id, x.r#gen), &primitive, 0);
                    }
                    Err(e ) => {eprintln!("<error resolving object>: {}", e)}
                };
            }
            Primitive::Name (ref x) => {self.ui.text(format!("/{}", x)); log::debug!("Name: {}", x);}

        }
    }

    pub fn view_dict(&self, dict: &Dictionary) {
        let mut id = 0;
        //log::debug!("Dictionary with {} entries", dict.len());
        for (key, val) in dict.iter() {
            self.attr(key, val, id);
            id += 1;
        }
        if dict.len() == 0 {
            self.ui.text("<No entries in dictionary>");     
        }
    }

    /// Note: the point with `id` is just that ImGui needs some unique string identifier for each
    /// tree node on the same level.
    pub fn attr(&self, name: &str, val: &Primitive, id: i32) {
        let _name = format!("<{}> <{}>", name, val.get_debug_name());
        //self.ui.tree_node(format!("{}", id))
        //    .map(|_tn| { _tn.set_label(_name); self.view_primitive(val)});
        let id_str: &str = &format!("{}", id);
        self.ui.tree_node_config(id_str)
            .label::<&str,_>(&_name)
            .push()
            .map(|_tn| self.view_primitive(val));
            //.build(|| self.view_primitive(val));
    }
}
