use std::{collections::BTreeMap, fs::OpenOptions, io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write}, path::PathBuf};

const KEY_VAL_HEADER_LEN: u32 = 4;

              // (key (byte), (file_id, value_pos, value_len))
type KeyDir = BTreeMap<Vec<u8>,(u64,u64,u32)>;
type Result<T> = std::result::Result<T, std::io::Error>;

const MAX_DATA_FILE_BYTES: u64 = 1024 * 1024; // 1 MiB

pub struct Bitcask{
    log: Log,
    keydir: KeyDir
}

impl Bitcask {
    pub fn new(path:PathBuf) -> Result<Self>{
        let mut log = Log::new(path)?;
        let keydir = log.load_index()?;
        Ok(Self { log, keydir })
    }

    fn write_and_index(
        log: &mut Log,
        keydir: &mut KeyDir,
        key: &[u8],
        value: &[u8],
    ) -> Result<()> {
        let (file_id, offset, total_len) = log.write_entry(key, Some(value))?;
        let value_len = value.len() as u32;
        keydir.insert(
            key.to_vec(),
            (file_id, offset + total_len as u64 - value_len as u64, value_len),
        );
        Ok(())
    }

    pub fn set(&mut self, key :&[u8], value : Vec<u8>) -> Result<()>{

        // +-------------+-------------+----------------+----------------+
        // | key len(4)    val len(4)     key(varint)       val(varint)  |
        // +-------------+-------------+----------------+----------------+
        Self::write_and_index(&mut self.log, &mut self.keydir, key, &value)?;
        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some((file_id, value_pos, value_len)) = self.keydir.get(key) {
            let val = self.log.read_value(*file_id, *value_pos, *value_len)?;
            return Ok(Some(val));
        }

        Ok(None)
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        let _ = self.log.write_entry(key, None)?;
        self.keydir.remove(key);
        Ok(())
    }

    pub fn merge(&mut self) -> Result<()> {
        let mut merge_log_path = self.log.dir_path.clone();
        merge_log_path.set_extension("merge");
        let mut merge_log = Log::new(merge_log_path)?;
        let mut new_keydir = KeyDir::new();

        for (key, (file_id, value_pos, value_len)) in self.keydir.iter() {

            let value = self.log.read_value(*file_id, *value_pos, *value_len)?;
            Self::write_and_index(&mut merge_log, &mut new_keydir, key, &value)?;
        }

        self.log = merge_log;
        self.keydir = new_keydir;
        Ok(())
    }
}


pub struct Log {
    dir_path: PathBuf,
    base_name: String,
    active_id: u64,
    active_file: std::fs::File,
    active_size: u64,
    read_files: BTreeMap<u64, std::fs::File>,
}

impl Log {
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let dir = path.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();
        let base_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid path"))?
            .to_string();

        let mut ids: Vec<u64> = std::fs::read_dir(&dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let prefix = format!("{}.", base_name);
                let suffix = ".data";
                if name.starts_with(&prefix) && name.ends_with(suffix) {
                    let id_str = &name[prefix.len()..name.len() - suffix.len()];
                    id_str.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .collect();

        ids.sort_unstable();

        let active_id = ids.last().copied().unwrap_or(0);
        let active_path = Self::data_file_path(&dir, &base_name, active_id);
        let active_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&active_path)?;
        let active_size = active_file.metadata()?.len();

        let mut read_files = BTreeMap::new();
        for id in ids.into_iter() {
            let read_path = Self::data_file_path(&dir, &base_name, id);
            let read_file = OpenOptions::new().read(true).open(&read_path)?;
            read_files.insert(id, read_file);
        }
        if !read_files.contains_key(&active_id) {
            let read_file = OpenOptions::new().read(true).open(&active_path)?;
            read_files.insert(active_id, read_file);
        }

        Ok(Self {
            dir_path: dir,
            base_name,
            active_id,
            active_file,
            active_size,
            read_files,
        })
    }

    fn data_file_path(dir: &PathBuf, base_name: &str, id: u64) -> PathBuf {
        dir.join(format!("{}.{}.data", base_name, id))
    }

    fn rotate_if_needed(&mut self, entry_len: u32) -> Result<()> {
        if self.active_size + entry_len as u64 <= MAX_DATA_FILE_BYTES {
            return Ok(());
        }

        self.active_id = self.active_id.saturating_add(1);
        let new_path = Self::data_file_path(&self.dir_path, &self.base_name, self.active_id);
        self.active_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&new_path)?;
        self.active_size = 0;

        let read_file = OpenOptions::new().read(true).open(&new_path)?;
        self.read_files.insert(self.active_id, read_file);
        Ok(())
    }

    fn load_index(&mut self) -> Result<KeyDir>{
        let mut buf_len = [0u8; KEY_VAL_HEADER_LEN as usize];
        let mut keydir = KeyDir::new();
        for (file_id, path) in self
            .read_files
            .keys()
            .cloned()
            .map(|id| (id, Self::data_file_path(&self.dir_path, &self.base_name, id)))
            .collect::<Vec<_>>()
        {
            let mut file = OpenOptions::new().read(true).open(&path)?;
            let file_len = file.metadata()?.len();
            let mut reader = BufReader::new(&mut file);
            let mut pos = reader.seek(SeekFrom::Start(0))?;

            while pos < file_len {
                let read_one = || -> Result<(Vec<u8>, u64, Option<u32>)> {
                    reader.read_exact(&mut buf_len)?;
                    let key_len = u32::from_be_bytes(buf_len);
                    reader.read_exact(&mut buf_len)?;
                    let value_lent_or_tombstone = match i32::from_be_bytes(buf_len) {
                        l if l >= 0 => Some(l as u32),
                        _ => None,
                    };
                    let value_pos = pos + KEY_VAL_HEADER_LEN as u64 * 2 + key_len as u64;
                    let mut key = vec![0u8; key_len as usize];
                    reader.read_exact(&mut key)?;

                    if let Some(value_len) = value_lent_or_tombstone {
                        reader.seek_relative(value_len as i64)?;
                    }

                    Ok((key, value_pos, value_lent_or_tombstone))
                }();

                match read_one {
                    Ok((key, value_pos, Some(value_len))) => {
                        keydir.insert(key, (file_id, value_pos, value_len));
                        pos = value_pos + value_len as u64;
                    }
                    Ok((key, value_pos, None)) => {
                        keydir.remove(&key);
                        pos = value_pos;
                    }
                    Err(err) => return Err(err.into()),
                }
            }
        }

        Ok(keydir)
    }

    fn write_entry(&mut self, key: &[u8], value: Option<&[u8]>) -> Result<(u64, u64, u32)>{
        let key_len = key.len() as u32;
        let value_len = value.map_or(0, |v| v.len() as u32);
        let value_len_or_tomestone = value.map_or(-1, |v| v.len() as i32);
        let total_len = KEY_VAL_HEADER_LEN * 2 + key_len + value_len;
        self.rotate_if_needed(total_len)?;
        let offset = self.active_size;
        let mut writer = BufWriter::with_capacity(total_len as usize, &mut self.active_file);

        writer.write_all(&key_len.to_be_bytes())?;
        writer.write_all(&value_len_or_tomestone.to_be_bytes())?;
        writer.write_all(key)?;
        if let Some(value) = value {
            writer.write_all(value)?;
        }
        writer.flush()?;
        self.active_size = self.active_size.saturating_add(total_len as u64);
        Ok((self.active_id, offset, total_len))
    }


    fn read_value(&mut self, file_id: u64, value_pos: u64, value_len: u32) -> Result<Vec<u8>> {
        let mut value = vec![0; value_len as usize];
        let file = self.read_files.get_mut(&file_id).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "data file not found")
        })?;
        file.seek(SeekFrom::Start(value_pos))?;
        file.read_exact(&mut value)?;
        Ok(value)
    }
}



#[cfg(test)]
mod tests {
    use super::{Bitcask, Log, Result, KEY_VAL_HEADER_LEN, MAX_DATA_FILE_BYTES};

    #[test]
    fn test_log_read_write() -> Result<()> {
        let path = std::env::temp_dir()
            .join("sqldb-disk-engine-log-test1")
            .join("log");

        let mut log = Log::new(path.clone())?;
        log.write_entry(b"a", Some(b"val1"))?;
        log.write_entry(b"b", Some(b"val2"))?;
        log.write_entry(b"c", Some(b"val3"))?;

        // rewrite
        log.write_entry(b"a", Some(b"val5"))?;
        // delete
        log.write_entry(b"c", None)?;

        let keydir = log.load_index()?;
        assert_eq!(2, keydir.len());

        path.parent().map(|p| std::fs::remove_dir_all(p));

        Ok(())
    }

    #[test]
    fn test_log_reopen() -> Result<()> {
        let path = std::env::temp_dir()
            .join("sqldb-disk-engine-log-test2")
            .join("log");

        {
            let mut log = Log::new(path.clone())?;
            log.write_entry(b"a", Some(b"val1"))?;
            log.write_entry(b"b", Some(b"val2"))?;
            log.write_entry(b"c", Some(b"val3"))?;
            log.write_entry(b"d", Some(b"val4"))?;
            log.write_entry(b"d", None)?;

        }

        let mut log = Log::new(path.clone())?;
        let keydir = log.load_index()?;
        assert_eq!(3, keydir.len());

        path.parent().map(|p| std::fs::remove_dir_all(p));

        Ok(())
    }

    #[test]
    fn test_point_opt() -> Result<()> {
        let path = std::env::temp_dir().join("Bitcask-test").join("log");
        let mut eng = Bitcask::new(path.clone())?;

        assert_eq!(eng.get(b"not exist")?, None);

        eng.set(b"aa", vec![1, 2, 3, 4])?;
        assert_eq!(eng.get(b"aa")?, Some(vec![1, 2, 3, 4]));

        eng.set(b"aa", vec![5, 6, 7, 8])?;
        assert_eq!(eng.get(b"aa")?, Some(vec![5, 6, 7, 8]));

        eng.delete(b"aa")?;
        assert_eq!(eng.get(b"aa")?, None);

        assert_eq!(eng.get(b"")?, None);
        eng.set(b"", vec![])?;
        assert_eq!(eng.get(b"")?, Some(vec![]));

        eng.set(b"cc", vec![5, 6, 7, 8])?;
        assert_eq!(eng.get(b"cc")?, Some(vec![5, 6, 7, 8]));

        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }

    #[test]
    fn test_merge() -> Result<()> {
        let path = std::env::temp_dir()
            .join("Bitcask-merge-test")
            .join("log");

        let mut eng = Bitcask::new(path.clone())?;

        eng.set(b"a", b"value1".to_vec())?;
        eng.set(b"b", b"value2".to_vec())?;
        eng.set(b"c", b"value3".to_vec())?;
        eng.delete(b"a")?;
        eng.delete(b"b")?;
        eng.delete(b"c")?;

        eng.merge()?;

        eng.set(b"a", b"value1".to_vec())?;
        eng.set(b"b", b"value2".to_vec())?;
        eng.set(b"c", b"value3".to_vec())?;

        let val = eng.get(b"a")?;
        assert_eq!(b"value1".to_vec(), val.unwrap());

        let val = eng.get(b"b")?;
        assert_eq!(b"value2".to_vec(), val.unwrap());

        let val = eng.get(b"c")?;
        assert_eq!(b"value3".to_vec(), val.unwrap());

        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }

    #[test]
    fn test_multi_file_rotation_and_reopen() -> Result<()> {
        let path = std::env::temp_dir()
            .join("Bitcask-multi-file-test")
            .join("log");

        let large_value = vec![42u8; MAX_DATA_FILE_BYTES as usize];
        let large_value2 = vec![7u8; MAX_DATA_FILE_BYTES as usize];

        {
            let mut eng = Bitcask::new(path.clone())?;
            eng.set(b"k1", large_value.clone())?;
            eng.set(b"k2", large_value2.clone())?;

            let got_k1 = eng.get(b"k1")?;
            assert_eq!(got_k1.as_deref(), Some(large_value.as_slice()));

            let got_k2 = eng.get(b"k2")?;
            assert_eq!(got_k2.as_deref(), Some(large_value2.as_slice()));
        }

        let mut eng = Bitcask::new(path.clone())?;
        let got_k1 = eng.get(b"k1")?;
        assert_eq!(got_k1.as_deref(), Some(large_value.as_slice()));

        let got_k2 = eng.get(b"k2")?;
        assert_eq!(got_k2.as_deref(), Some(large_value2.as_slice()));

        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }

    #[test]
    fn test_multi_file_delete_across_rotation() -> Result<()> {
        let path = std::env::temp_dir()
            .join("Bitcask-multi-file-delete-test")
            .join("log");

        let large_value = vec![1u8; MAX_DATA_FILE_BYTES as usize];

        {
            let mut eng = Bitcask::new(path.clone())?;
            eng.set(b"tombstone-key", large_value)?;
            eng.delete(b"tombstone-key")?;
        }

        let mut eng = Bitcask::new(path.clone())?;
        assert_eq!(eng.get(b"tombstone-key")?, None);

        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }

    #[test]
    fn test_multi_file_delete_merge_reload() -> Result<()> {
        let path = std::env::temp_dir()
            .join("Bitcask-multi-file-merge-reload-test")
            .join("log");

        let v1 = vec![1u8; MAX_DATA_FILE_BYTES as usize];
        let v2 = vec![2u8; MAX_DATA_FILE_BYTES as usize];
        let v3 = vec![3u8; MAX_DATA_FILE_BYTES as usize];

        {
            let mut eng = Bitcask::new(path.clone())?;
            eng.set(b"k1", v1.clone())?;
            eng.set(b"k2", v2.clone())?;
            eng.set(b"k3", v3.clone())?;

            eng.delete(b"k2")?;
            eng.merge()?;

            assert_eq!(eng.get(b"k2")?, None);
            assert_eq!(eng.get(b"k1")?.as_deref(), Some(v1.as_slice()));
            assert_eq!(eng.get(b"k3")?.as_deref(), Some(v3.as_slice()));

        }

        let mut eng = Bitcask::new(path.clone())?;
        assert_eq!(eng.get(b"k2")?, None);
        assert_eq!(eng.get(b"k1")?.as_deref(), Some(v1.as_slice()));
        assert_eq!(eng.get(b"k3")?.as_deref(), Some(v3.as_slice()));

        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }

    #[test]
    fn test_many_keys_multi_file_merge() -> Result<()> {
        let path = std::env::temp_dir()
            .join("Bitcask-many-keys-multi-file-merge-test")
            .join("log");

        let mut eng = Bitcask::new(path.clone())?;
        let value_size = 1024;
        let value = vec![9u8; value_size];

        let approx_entry = (KEY_VAL_HEADER_LEN as usize * 2) + 8 + value_size;
        let target_entries = (MAX_DATA_FILE_BYTES as usize / approx_entry) * 3;

        for i in 0..target_entries {
            let key = format!("key-{}", i);
            eng.set(key.as_bytes(), value.clone())?;
        }

        for i in (0..target_entries).step_by(target_entries / 5) {
            let key = format!("key-{}", i);
            eng.delete(key.as_bytes())?;
        }

        eng.merge()?;

        for i in 0..target_entries {
            let key = format!("key-{}", i);
            let val = eng.get(key.as_bytes())?;
            if i % (target_entries / 5) == 0 {
                assert_eq!(val, None);
            } else {
                assert_eq!(val.as_deref(), Some(value.as_slice()));
            }
        }


        path.parent().map(|p| std::fs::remove_dir_all(p));
        Ok(())
    }
}
