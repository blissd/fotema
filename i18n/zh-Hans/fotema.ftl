library-page = 图库
years-album = 年
months-album = 月
all-album = 日
videos-album = 视频
selfies-album = 自拍
animated-album = 动态
folders-album = 文件夹
folder-album = 文件夹
places-page = 地方
people-page = 人物
people-page-status-off =
    .title = 开启人脸识别？
    .description = { -app-name } 可以自动检测人脸，但是此行动会消耗很长时间。确定要开启？
    .notice = { -app-name } 必须得下载 45 MB 的数据才能够人脸识别。
    .enable = 开启
about-opensource = 开源项目
about-translator-credits = Geeson Wan (wang14240@gmail.com)
viewer-info-tooltip = 显示属性
viewer-faces-menu =
    .tooltip = 人脸菜单
    .restore-ignored = 恢复所有已忽略人脸
    .ignore-unknown = 忽略所有未知人脸
    .scan = 识别更多人脸
viewer-next =
    .tooltip = 下一个
viewer-previous =
    .tooltip = 上一个
viewer-play =
    .tooltip = 播放/暂停
viewer-skip-forward-10-seconds =
    .tooltip = 前进 10 秒
viewer-skip-backwards-10-seconds =
    .tooltip = 后退 10 秒
viewer-mute =
    .tooltip = 静音/取消静音
viewer-error-failed-to-load = 无法加载
month-thumbnail-label =
    { $month ->
        [1] { $year } 年 1 月
        [2] { $year } 年 2 月
        [3] { $year } 年 3 月
        [4] { $year } 年 4 月
        [5] { $year } 年 5 月
        [6] { $year } 年 6 月
        [7] { $year } 年 7 月
        [8] { $year } 年 8 月
        [9] { $year } 年 9 月
        [10] { $year } 年 10 月
        [11] { $year } 年 11 月
        [12] { $year } 年 12 月
       *[other] { $year } 年
    }
viewer-convert-all-description = 此视频播放前必须转换格式。此操作只会运行一次，但是会需要一些时间。
viewer-convert-all-button = 转换所有不兼容的视频
people-page-status-no-people =
    .title = 未识别任何人脸
    .description =
        { -app-name } 启动后会自动在相片中识别人脸
        请为识别的人物命名，以便 { -app-name } 可以为每人创建一本相册。
viewer-error-missing-file =
    无法显示文件，该文件已丢失：
    { $file_name }
viewer-error-missing-path = 文件路径不存在于数据库
infobar-folder = 文件夹
    .tooltip = 打开所在文件夹
infobar-file-name = 文件名
infobar-file-created = 文件创建
infobar-file-modified = 文件修改
infobar-file-size = 文件大小
infobar-file-format = 格式
infobar-originally-created = 最初创建
infobar-originally-modified = 最初编辑
infobar-video-duration = 时长
infobar-video-container-format = 媒体容器格式
infobar-video-codec = 视频编解码
infobar-audio-codec = 音频编解码
infobar-dimensions = 尺寸
people-set-face-thumbnail = 用作缩略图
people-set-name = 设置名称
people-person-search =
    .placeholder = 人名
people-face-ignore = 忽略
people-not-this-person = 不是 { $name }
prefs-title = 偏好
prefs-albums-section = 相册
    .description = 配置相册。
prefs-albums-selfies = 自拍
    .subtitle = 将 iOS 设备的自拍放入单独的相册中。重启 { -app-name } 即可应用更改。
prefs-albums-chronological-sort = 排序顺序
    .subtitle = 相册按时间顺序排序
    .ascending = 升序
    .descending = 降序
prefs-processing-section = 相片与视频处理
    .description = 配置相片与视频处理功能。
prefs-processing-face-detection = 人脸识别
    .subtitle = 检测并识别已命名的人脸。此行动会消耗很长时间。
primary-menu-about = 关于 { -app-name }
person-menu-rename = 重命名人物
person-menu-delete = 删除人物
person-delete-dialog =
    .heading = 删除人物？
    .body = 相关相片和视频不会被删除。
    .cancel-button = 取消
    .delete-button = 删除
person-rename-dialog =
    .heading = 重命名人物？
    .placeholder = 新名称
    .cancel-button = 取消
    .rename-button = 重命名
banner-convert-videos = 转换视频中。
progress-idle = 空闲。
banner-button-stop =
    .label = 停止
    .tooltip = 停止所有背景任务。
banner-stopping = 正在停止任务…
primary-menu-preferences = 偏好
banner-face-thumbnails = 生成人脸缩略图
onboard-select-pictures =
    .title = 欢迎来到 { -app-name }。
    .description =
        请选择您存放相片的文件夹路径。

        如果您使用过 { -app-name } 可以自动检测相片库的旧版本，请使用根之前一致的文件夹路径来避免相片被重新处理。
    .button = 选择文件夹路径
-app-name = Fotema
prefs-processing-motion-photos = 动态相片
    .subtitle = 检测 Android 动态相片并且提取视频。
prefs-library-section =
    .title = 图库
    .description =
        配置图库路径
        警告：改变图库路径可能会让 { -app-name } 重新处理所有相片。
prefs-library-section-pictures-dir =
    .title = 相片路径
    .tooltip = 选择线片路径。
progress-metadata-photos = 正在处理相片元数据。
progress-metadata-videos = 正在处理视频元数据。
progress-thumbnails-photos = 正在生成相片缩略图。
progress-thumbnails-videos = 正在生成视频缩略图。
progress-thumbnails-faces = 正在生成人脸缩略图。
progress-convert-videos = 正在转换视频。
progress-motion-photo = 正在处理动态相片。
progress-detect-faces-photos = 正在人脸识别相片。
progress-recognize-faces-photos = 正在识别相片中的人物。
banner-scan-library = 正在扫描相库。
banner-metadata-photos = 正在处理相片元数据。
banner-metadata-videos = 正在处理视频元数据。
banner-thumbnails-photos = 正在生成相片缩略图。此操作可能需要一阵时间。
banner-thumbnails-videos = 正在生成视频缩略图。此操作可能需要一阵时间。
banner-clean-photos = 相片数据库维护。
banner-clean-videos = 视频数据库维护。
banner-extract-motion-photos = 正在处理动态相片。
banner-detect-faces-photos = 正在人脸识别相片。此操作可能需要一阵时间。
banner-recognize-faces-photos = 正在识别相片中的人物。此操作可能需要一阵时间。
