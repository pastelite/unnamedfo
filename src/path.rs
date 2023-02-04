use std::{os::windows::prelude::FileExt, path::PathBuf};

pub struct FilePath {
    data: Vec<String>,
}

impl From<&str> for FilePath {
    fn from(path: &str) -> Self {
        let path = path.to_owned();
        Self {
            data: path
                .split(|char| (char == '/' || char == '\\'))
                .filter(|s| s != &"." && s != &"")
                .map(|e| e.to_owned())
                .collect(),
        }
    }
}

impl From<&PathBuf> for FilePath {
    fn from(path: &PathBuf) -> Self {
        let path = path.to_str().unwrap();
        Self {
            data: path
                .split(|char| (char == '/' || char == '\\'))
                .filter(|s| s != &"." && s != &"")
                .map(|e| e.to_owned())
                .collect(),
        }
    }
}

impl FilePath {
    pub fn get_path(&self) -> String {
        let joined = "./".to_owned() + &self.data.join("/");
        return joined;
    }

    pub fn get_name(&self) -> String {
        return self.data.last().unwrap().to_string();
    }

    pub fn cut(mut self, paths: &FilePath) -> Self {
        // remove the self.data before the paths
        let mut i = 0;
        for path in self.data.iter() {
            println!("{} {} {}", path, &paths.data.last().unwrap(), i);
            if path.eq(paths.data.last().unwrap()) {
                break;
            }
            i += 1;
        }
        self.data.drain(0..(i + 1));
        return self;
    }

    pub fn append(&mut self, path: &str) {
        let path = Self::from(path);
        self.data.append(&mut path.data.clone());
    }

    pub fn exists(&self) -> bool {
        let path = PathBuf::from(self.get_path());
        return path.exists();
    }
}

#[test]
fn test() {
    assert_eq!(
        vec![String::from("test"), String::from("c")],
        FilePath::from("./test/c").data
    );
    assert_eq!("./test/c", &FilePath::from("./test/c").get_path());
    assert_eq!("./test/c", &FilePath::from("/test\\c").get_path());
    assert_eq!(
        "./c",
        &FilePath::from("/test\\c")
            .cut(&FilePath::from("test"))
            .get_path()
    );
    assert_eq!(
        "./d",
        &FilePath::from("/a/b/c/d")
            .cut(&FilePath::from("c"))
            .get_path()
    );
    let mut fp1 = FilePath::from("/a/b");
    fp1.append("c/d.txt");
    assert_eq!("./a/b/c/d.txt", &fp1.get_path());
}
