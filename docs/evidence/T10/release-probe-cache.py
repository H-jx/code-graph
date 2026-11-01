"""仅建议丢弃本任务 WSL 构建缓存的干净文件页；不删除文件或修改系统缓存设置。"""
import os
from pathlib import Path

root = Path('/root/.cache/project-knowledge-linux-probe').resolve(strict=True)
count = 0
for directory, _, files in os.walk(root, followlinks=False):
    for name in files:
        path = Path(directory) / name
        if path.is_symlink() or not path.is_file():
            continue
        try:
            descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                os.posix_fadvise(descriptor, 0, 0, os.POSIX_FADV_DONTNEED)
                count += 1
            finally:
                os.close(descriptor)
        except OSError:
            pass
print('{"ownedCacheFilesAdvised":' + str(count) + '}')
