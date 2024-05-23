
## Terms
# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Library

# Title for years album.
years-album = Year

# Title for months album.
months-album = Month

# Title for all photos/videos album.
all-album = Day

# Title for video album.
videos-album = Videos

# Title for album of selfies.
selfies-album = Selfies

# Title for album of iOS live photos and Android motion photos.
animated-album = Animated

# Title for album showing all folders.
folders-album = Folders

# Title for album showing contents of one folder.
folder-album = Folder

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
month-thumbnail-label = { $month ->
  [1] January
  [2] February
  [3] March
  [4] April
  [5] May
  [6] June
  [7] July
  [8] August
  [9] September
  [10] October
  [11] November
  [12] December
  *[other] Other
  } {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Open Source Projects

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .open - tooltip text for open folder action button.
infobar-folder = Folder
  .open = Open Containing Folder

# File name of photo or video
infobar-file-name = File Name

# File creation timestamp from file system metadata.
infobar-file-created = File Created

# File modification timestamp from file system metadata.
infobar-file-modified = File Modified

# File size file system metadata.
infobar-file-size = File Size

# File format, such as "JPEG" or "PNG".
infobar-file-format = Format

# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Originally Created

# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Originally Modified

# Duration (HH:MM) of video.
infobar-video-duration = Duration

# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Container Format

# Video codec, such as "AV1".
infobar-video-codec = Video Codec

# Audio codec, such as "OPUS".
infobar-audio-codec = Audio Codec

# Width and height of photo or video.
infobar-dimensions = Dimensions

## Preferences

# Title of section of preferences for views
prefs-views-section = Views
  .description = Show or hide sidebar views

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfies
  .subtitle = Shows a separate view for selfies taken on iOS devices. Restart {-app-name} to apply.

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = Processing photo metadata.

# Extracting details from video container metadata
progress-metadata-videos = Processing video metadata.

# Generating thumbnails from photos
progress-thumbnails-photos = Generating photo thumbnails.

# Generating thumbnails from videos
progress-thumbnails-videos = Generating video thumbnails.

# Transcoding videos to a compatible format
progress-convert-videos = Converting videos.

# Not doing any background work
progress-idle = Idle.

## Notification banner for background tasks
# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = Scanning file system for photos.

# Scanning file system for new videos
banner-scan-videos = Scanning file system for videos.

# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Processing photo metadata.

# Processing new videos to extract metadata from video container.
banner-metadata-videos = Processing video metadata.

# Generating thumbnails for all photos.
banner-thumbnails-photos = Generating photo thumbnails. This will take a while.

# Generating thumbnails for all videos.
banner-thumbnails-videos = Generating video thumbnails. This will take a while.

# Updating the database to remove details of absent photos.
banner-clean-photos = Photo database maintenance.

# Updating the database to remove details of absent videos.
banner-clean-videos = Video database maintenance.
