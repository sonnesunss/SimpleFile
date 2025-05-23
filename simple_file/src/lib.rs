use libc::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, close, open, read, write};
use std::ffi::CString;
use std::io;
use std::io::{Read, Write};
use std::path::Path;

/////////表示文件打开模式////////////////////
#[derive(Clone, Copy)]
pub enum OpenMode {
    Read,
    Write,
    ReadWrite,
}

/////////////////////////////////////////////
#[allow(dead_code)]
pub struct File {
    fd: i32,
}

const INVALID_FD: i32 = -1;
const DEFAULT_FILE_PERMSSIONS: i32 = 0o644; // 默认文件权限
/////////////////////////////////////////////

/*

后面的一切简单起见，封装POSX内的对应函数实现之

在POSIX中，打开文件使用open syscall. 需要为其传递文件路径、打开模式等

https://pubs.opengroup.org/onlinepubs/007904875/functions/open.html 这里是对open的细节描述
由于使用了这个POSIX 函数实现File，所以先看看这个函数的使用

1. 需要三个参数，文件路径，flags，mode，
    flags表示打开模式，只读，只写，可读写等等，挺多的
    mode表示如果文件被创建，指定其权限

    需要特别注意，文件路径，需要一个c语言类型的字符串，也就是尾巴有空字符的字符串，rust里面需要构建出这个字符串

2. 返回值，成功返回文件描述符，失败返回-1， 这也是之前创建常量INVALID_FD的原因

*/
impl File {
    /// step1: 构建c-style文件路径字符串
    /// step2: 组装打开模式
    /// step3: unsafe封装POSIX open函数
    /// step4: 返回结果File
    pub fn open<P: AsRef<Path>>(path: P, mode: OpenMode) -> io::Result<File> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid path, empty not allowed",
            ));
        }
        let c_style_str_path = CString::new(
            path.to_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid path"))?,
        )?;

        let flags = match mode {
            OpenMode::Read => O_RDONLY,
            OpenMode::Write => O_WRONLY | O_CREAT | O_TRUNC,
            OpenMode::ReadWrite => O_RDWR | O_CREAT,
        };

        let fd = unsafe { open(c_style_str_path.as_ptr(), flags, DEFAULT_FILE_PERMSSIONS) };

        if fd == INVALID_FD {
            return Err(io::Error::last_os_error());
        }

        Ok(File { fd })
    }

    /*
        实现read方法，同样通过封装posix read syscall实现
        注意这里的io::Result<T>其实是std::result::Result<T, E>的alias别名
        因为而io中的错误实在太常见了，简化目的创建了一个别名

        read(fd: i32, buf: *mut c_void, count: size_t) -> ssize_t
    */
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.fd == INVALID_FD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "File is closed",
            ));
        }

        let len = buf.len();
        let result = unsafe {
            // fd， 缓冲区，读取大小，字节为基本单位
            read(self.fd, buf.as_mut_ptr() as *mut _, len as libc::size_t)
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(result as usize)
    }

    /*
       实现写入方法， 依旧通过封装POSIX write syscall实现

       write(fd: i32, buf: *const c_void, count: size_t) -> ssize_t
    */
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.fd == INVALID_FD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "File is closed",
            ));
        }

        let len = buf.len();
        let result = unsafe { write(self.fd, buf.as_ptr() as *const _, len as libc::size_t) };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(result as usize)
    }
}

/*
    POSIX open函数是需要手动释放资源的， 所以也要有对等的rust实现
    Rust通过RAII（资源获取即初始化）进行自动的资源释放，通过实现Drop trat即可
*/
impl Drop for File {
    fn drop(&mut self) {
        if self.fd != INVALID_FD {
            unsafe {
                close(self.fd);
            }

            self.fd = INVALID_FD; // 避免重复关闭
        }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read(buf)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{File, OpenMode};
    use std::io::{self, Read, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_open_read() -> io::Result<()> {
        // 创建一个临时文件并写入内容
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Hello, world!")?;

        let file = File::open(temp_file.path(), OpenMode::Read)?;
        assert!(file.fd >= 0, "File descriptor should be valid");

        Ok(())
    }

    #[test]
    fn test_open_write() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let file = File::open(temp_file.path(), OpenMode::Write)?;
        assert!(file.fd >= 0, "File descriptor should be valid");

        Ok(())
    }

    #[test]
    fn test_open_read_write() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let file = File::open(temp_file.path(), OpenMode::ReadWrite)?;
        assert!(file.fd >= 0, "File descriptor should be valid");

        Ok(())
    }

    #[test]
    fn test_open_nonexistent_file() {
        let result = File::open("nonexistent.txt", OpenMode::Read);
        assert!(result.is_err(), "Opening nonexistent file should fail");
        if let Err(e) = result {
            assert_eq!(
                e.kind(),
                io::ErrorKind::NotFound,
                "Error should be NotFound"
            );
        }
    }

    #[test]
    fn test_open_invalid_path() {
        let result = File::open("", OpenMode::Read);
        assert!(result.is_err(), "Opening empty path should fail");
        if let Err(e) = result {
            assert_eq!(
                e.kind(),
                io::ErrorKind::InvalidInput,
                "Error should be InvalidInput"
            );
        }
    }

    // 测试读取
    #[test]
    fn test_read_content() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        let content = b"Hello, world!";
        temp_file.write_all(content)?;

        let mut file = File::open(temp_file.path(), OpenMode::Read)?;
        let mut buf = [0u8; 128];
        let n = file.read(&mut buf)?;
        assert_eq!(n, content.len(), "Should read exact number of bytes");
        assert_eq!(&buf[..n], content, "Read content should match");

        Ok(())
    }

    #[test]
    fn test_read_empty_file() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let mut file = File::open(temp_file.path(), OpenMode::Read)?;
        let mut buf = [0u8; 128];
        let n = file.read(&mut buf)?;
        assert_eq!(n, 0, "Reading empty file should return 0 bytes");

        Ok(())
    }

    #[test]
    fn test_read_empty_buffer() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"Some content")?;
        let mut file = File::open(temp_file.path(), OpenMode::Read)?;
        let mut buf = [];
        let n = file.read(&mut buf)?;
        assert_eq!(n, 0, "Reading with empty buffer should return 0");

        Ok(())
    }

    #[test]
    fn test_read_invalid_fd() {
        let mut file = File { fd: -1 }; // 手动构造无效文件描述符
        let mut buf = [0u8; 128];
        let result = file.read(&mut buf);
        assert!(result.is_err(), "Reading with invalid fd should fail");
        if let Err(e) = result {
            assert_eq!(
                e.kind(),
                io::ErrorKind::InvalidInput,
                "Error should be InvalidInput"
            );
        }
    }

    // 测试写入
    #[test]
    fn test_write_content() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let content = b"Hello, world!";
        let mut file = File::open(temp_file.path(), OpenMode::Write)?;
        let n = file.write(content)?;
        assert_eq!(n, content.len(), "Should write exact number of bytes");

        let mut std_file = std::fs::File::open(temp_file.path())?;
        let mut read_content = Vec::new();
        std_file.read_to_end(&mut read_content)?;
        assert_eq!(read_content, content, "Written content should match");

        Ok(())
    }

    #[test]
    fn test_write_invalid_fd() {
        let mut file = File { fd: -1 }; // 手动构造无效文件描述符
        let content = b"test";
        let result = file.write(content);
        assert!(result.is_err(), "Writing with invalid fd should fail");
        if let Err(e) = result {
            assert_eq!(
                e.kind(),
                io::ErrorKind::InvalidInput,
                "Error should be InvalidInput"
            );
        }
    }

    // 测试 Drop（自动关闭）
    #[test]
    fn test_drop_closes_fd() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let file = File::open(temp_file.path(), OpenMode::Read)?;
        let fd = file.fd;
        assert!(fd >= 0, "File descriptor should be valid");

        // 手动 drop 文件
        drop(file);

        // 尝试使用原始 fd 调用 read（应失败）
        let mut buf = [0u8; 128];
        let result =
            unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len() as libc::size_t) };
        assert_eq!(result, -1, "Reading closed fd should fail");
        let err = io::Error::last_os_error();

        eprintln!("Error kind: {:?}", err.kind());
        eprintln!("Actual error: {:?}", err);
        eprintln!("Raw OS error code: {:?}", err.raw_os_error());
        assert_eq!(
            err.raw_os_error(),
            Some(libc::EBADF),
            "Raw OS error should be EBADF (9), got {:?}",
            err.raw_os_error()
        );

        Ok(())
    }

    #[test]
    fn test_read_trait() -> io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        let content = b"Hello, world!";
        temp_file.write_all(content)?;

        let mut file = File::open(temp_file.path(), OpenMode::Read)?;
        let mut read_content = Vec::new();
        file.read_to_end(&mut read_content)?;
        assert_eq!(
            read_content, content,
            "Read trait should read correct content"
        );

        Ok(())
    }

    #[test]
    fn test_write_trait() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let content = b"Hello, world!";
        let mut file = File::open(temp_file.path(), OpenMode::Write)?;
        file.write_all(content)?;
        file.flush()?;

        let mut std_file = std::fs::File::open(temp_file.path())?;
        let mut read_content = Vec::new();
        std_file.read_to_end(&mut read_content)?;
        assert_eq!(
            read_content, content,
            "Write trait should write correct content"
        );

        Ok(())
    }
}
