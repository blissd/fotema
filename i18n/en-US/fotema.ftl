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

# Title for places page which shows photos overlayed onto a map.
places-page = Places

# Title for people page which shows an album of faces.
people-page = People

# Status page shown for people album when face detection is disabled.
people-page-status-off =
  .title = Enable face detection?
  .description = { -app-name } can automatically detect faces and recognize people. Do you want to enable this feature?
  .notice = { -app-name } must download about 45 megabytes of data to recognize faces and people.
  .enable-mobile = Enable for mobile
  .enable-desktop = Enable for desktop

# Status page shown for people album when no people are found.
people-page-status-no-people =
  .title = No people found
  .description = { -app-name } will look for faces in new photos when launched.
  If you've told { -app-name } that a face is for a person you know, then { -app-name }
  will also look for new photos of that person.

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label = { $month ->
   [1] January {$year}
   [2] February {$year}
   [3] March {$year}
   [4] April {$year}
   [5] May {$year}
   [6] June {$year}
   [7] July {$year}
   [8] August {$year}
   [9] September {$year}
   [10] October {$year}
   [11] November {$year}
   [12] December {$year}
  *[other] {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Open Source Projects

# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits =
  David Bliss <hello@fotema.app>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Show properties

viewer-faces-menu =
  .tooltip = Faces menu
  .restore-ignored = Restore all ignored faces
  .ignore-unknown = Ignore all unknown faces
  .scan = Scan for more faces

# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
  .tooltip = Next

# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
  .tooltip = Previous

# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
  .tooltip = Play/Pause

# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
  .tooltip = Skip Forward 10 Seconds

# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
  .tooltip = Skip Backwards 10 Seconds

# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
  .tooltip = Mute/Unmute

# Convert all incompatible videos description.
viewer-convert-all-description = This video must be converted before it can be played. This only needs to happen once, but it takes a while to convert a video.

# Button to convert all incompatible videos.
viewer-convert-all-button = Convert all incompatible videos

# Viewer failed to load an image or video.
viewer-error-failed-to-load = Failed to load

# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file = Cannot display file because it is missing:
  {$file_name}

# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = File path not present in database

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Folder
  .tooltip = Open Containing Folder

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

## Faces and People

# Menu item to mark a face as the most import face for a person
# and to use it as a thumbnail
people-set-face-thumbnail = Use as thumbnail

# Menu item to mark associate a face with a person.
people-set-name = Set name

# Placeholder text for text entry bar used to name a new person or
# search for an existing person.
people-person-search =
        .placeholder = Person name

# Menu item to ignore a face match because it is a random face or not a face.
people-face-ignore = Ignore

# Menu item to dis-associate a face with a person.
# Variables:
#   $name - name of person
people-not-this-person = Not { $name }

## Preferences

# Title of preferences dialog
prefs-title = Preferences

# Title of section of preferences for views
prefs-views-section = Views
  .description = Show or hide sidebar views

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfies
  .subtitle = Shows a separate view for selfies taken on iOS devices. Restart {-app-name} to apply.

prefs-views-faces = Face Detection
  .subtitle = Enable face detection.

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

# Extracting motion photo videos
progress-motion-photo = Processing motion photos.

# Detect faces from photos
progress-detect-faces-photos = Detecting faces in photos.

# Recognize faces in photos as known people
progress-recognize-faces-photos = Recognizing people in photos.

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

# Extracting video component from Android motion photos
banner-extract-motion-photos = Processing motion photos.

# Detect and extract faces from photos
banner-detect-faces-photos = Detecting faces in photos. This will take a while.

# Recognize faces as people
banner-recognize-faces-photos = Recognizing people in photos. This will take a while.

## Primary menu

# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = Preferences

# Menu item to show "about" dialog
primary-menu-about = About {-app-name}

## Person menu

# Menu item to rename a person
person-menu-rename = Rename person

# Menu item to delete a person
person-menu-delete = Delete person

# Person delete dialog
person-delete-dialog =
  .heading = Delete person?
  .body = No pictures or videos will be deleted.
  .cancel-button = Cancel
  .delete-button = Delete

# Person delete dialog
person-rename-dialog =
  .heading = Rename person?
  .placeholder = New name
  .cancel-button = Cancel
  .rename-button = Rename
