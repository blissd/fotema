# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later

## Terms

# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = लाइब्रेरी

# Title for years album.
years-album = वर्ष

# Title for months album.
months-album = महीना

# Title for all photos/videos album.
all-album = दिन

# Title for video album.
videos-album = वीडियो

# Title for album of selfies.
selfies-album = सेल्फी

# Title for album of iOS live photos and Android motion photos.
animated-album = सजीवित

# Title for album showing all folders.
folders-album = फोल्डर

# Title for album showing contents of one folder.
folder-album = फोल्डर

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label = { $month ->
   [1] जनवरी {$year}
   [2] फरवरी {$year}
   [3] मार्च {$year}
   [4] अप्रैल {$year}
   [5] मई {$year}
   [6] जून {$year}
   [7] जुलाई {$year}
   [8] अगस्त {$year}
   [9] सितम्बर {$year}
   [10] अक्टूबर {$year}
   [11] नवम्बर {$year}
   [12] दिसम्बर {$year}
  *[other] {$year}
}

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = मुक्त स्रोत परियोजनाएं

# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits =
  Scrambled777 <weblate.scrambled777@simplelogin.com>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = गुण दिखाएं

# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
  .tooltip = अगला

# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
  .tooltip = पिछला

# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
  .tooltip = चलाएं/रोकें

# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
  .tooltip = 10 सेकंड आगे जाएं

# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
  .tooltip = 10 सेकंड पीछे जाएं

# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
  .tooltip = मूक/अमूक

# Convert all incompatible videos description
viewer-convert-all-description = इस वीडियो को चलाने से पहले इसे परिवर्तित किया जाना चाहिए। ऐसा केवल एक बार होने की आवश्यकता है, लेकिन किसी वीडियो को परिवर्तित करने में कुछ समय लगता है।

# Button to convert all incompatible videos.
viewer-convert-all-button = सभी असंगत वीडियो परिवर्तित करें

# Viewer failed to load an image or video.
viewer-error-failed-to-load = लोड करने में विफल

# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file = फाइल प्रदर्शित नहीं हो सकती क्योंकि वह अनुपलब्ध है:
  {$file_name}

# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = फाइल पथ डेटाबेस में मौजूद नहीं है

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = फोल्डर
  .tooltip = धारक फोल्डर खोलें

# File name of photo or video
infobar-file-name = फाइल नाम

# File creation timestamp from file system metadata.
infobar-file-created = फाइल निर्मित

# File modification timestamp from file system metadata.
infobar-file-modified = फाइल संशोधित

# File size file system metadata.
infobar-file-size = फाइल आकार

# File format, such as "JPEG" or "PNG".
infobar-file-format = प्रारूप

# File creation timestamp from image or video embedded metadata.
infobar-originally-created = मूलतः निर्मित

# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = मूलतः संशोधित

# Duration (HH:MM) of video.
infobar-video-duration = अवधि

# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = कंटेनर प्रारूप

# Video codec, such as "AV1".
infobar-video-codec = वीडियो कोडेक

# Audio codec, such as "OPUS".
infobar-audio-codec = ऑडियो कोडेक

# Width and height of photo or video.
infobar-dimensions = आयाम

## Preferences

# Title of section of preferences for views
prefs-views-section = दृश्य
  .description = पार्श्वपट्टी दृश्य दिखाएं या छुपाएं

# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = सेल्फी
  .subtitle = iOS उपकरणों पर ली गई सेल्फी के लिए एक अलग दृश्य दिखाता है। लागू करने के लिए {-app-name} पुनः आरंभ करें।

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = फोटो मेटाडेटा का प्रसंस्करण।

# Extracting details from video container metadata
progress-metadata-videos = वीडियो मेटाडेटा का प्रसंस्करण।

# Generating thumbnails from photos
progress-thumbnails-photos = फोटो थंबनेल उत्पन्न किया जा रहा है।

# Generating thumbnails from videos
progress-thumbnails-videos = वीडियो थंबनेल उत्पन्न किया जा रहा है।

# Transcoding videos to a compatible format
progress-convert-videos = वीडियो परिवर्तित किया जा रहा है।

# Extracting motion photo videos
progress-motion-photo = मोशन फोटो का प्रसंस्करण।

# Not doing any background work
progress-idle = निष्क्रिय।

## Notification banner for background tasks

# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = फोटो के लिए फाइल सिस्टम को स्कैन किया जा रहा है।

# Scanning file system for new videos
banner-scan-videos = वीडियो के लिए फाइल सिस्टम को स्कैन किया जा रहा है।

# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = फोटो मेटाडेटा का प्रसंस्करण।

# Processing new videos to extract metadata from video container.
banner-metadata-videos = वीडियो मेटाडेटा का प्रसंस्करण।

# Generating thumbnails for all photos.
banner-thumbnails-photos = फोटो थंबनेल उत्पन्न किया जा रहा है। इसमें कुछ समय लगेगा।

# Generating thumbnails for all videos.
banner-thumbnails-videos = वीडियो थंबनेल उत्पन्न किया जा रहा है। इसमें कुछ समय लगेगा।

# Updating the database to remove details of absent photos.
banner-clean-photos = फोटो डेटाबेस का रखरखाव।

# Updating the database to remove details of absent videos.
banner-clean-videos = वीडियो डेटाबेस का रखरखाव।

# Extracting video component from Android motion photos
banner-extract-motion-photos = मोशन फोटो का प्रसंस्करण।

## Primary menu

# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = प्राथमिकताएं

# Menu item to show "about" dialog
primary-menu-about = {-app-name} के बारे में
