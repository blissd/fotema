## Terms


# See https://projectfluent.org/fluent/guide/terms.html

-app-name = फोटेमा

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
# Title for places page which shows photos overlayed onto a map.
places-page = स्थान

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] जनवरी { $year }
        [2] फरवरी { $year }
        [3] मार्च { $year }
        [4] अप्रैल { $year }
        [5] मई { $year }
        [6] जून { $year }
        [7] जुलाई { $year }
        [8] अगस्त { $year }
        [9] सितम्बर { $year }
        [10] अक्टूबर { $year }
        [11] नवम्बर { $year }
        [12] दिसम्बर { $year }
       *[other] { $year }
    }

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = ओपन सोर्स परियोजनाएं
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = Scrambled777 <weblate.scrambled777@simplelogin.com>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = प्रॉपर्टी दिखाएं
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
viewer-error-missing-file =
    फाइल प्रदर्शित नहीं हो सकती क्योंकि वह अनुपलब्ध है:
    { $file_name }
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

# Title of preferences dialog
prefs-title = प्राथमिकताएं
# Title of section of preferences for views
prefs-views-section = दृश्य
    .description = पार्श्वपट्टी दृश्य दिखाएं या छुपाएं
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = सेल्फी
    .subtitle = iOS उपकरणों पर ली गई सेल्फी के लिए एक अलग दृश्य दिखाता है। लागू करने के लिए { -app-name } पुनः आरंभ करें।

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
primary-menu-about = { -app-name } के बारे में
people-set-name = नाम तय करें
banner-detect-faces-photos = फोटो में चेहरों का पता लगाया जा रहा है। इसमें थोड़ा समय लगेगा।
people-page = व्यक्ति
people-page-status-no-people =
    .title = कोई लोग नहीं मिले
    .description =
        { -app-name } शुरू होने पर नई फोटो में चेहरों की तलाश करेगी।
        अपनी फोटो में लोगों का नाम बताएं ताकि { -app-name } प्रत्येक व्यक्ति के लिए एक एल्बम बना सके।
viewer-faces-menu =
    .tooltip = चेहरे की मेनू
    .restore-ignored = सभी अनदेखे चेहरों को पुनर्स्थापित करें
    .ignore-unknown = सभी अनजान चेहरों को नजरअंदाज करें
    .scan = और चेहरों के लिए स्कैन करें
people-set-face-thumbnail = थंबनेल के रूप में उपयोग करें
people-person-search =
    .placeholder = व्यक्ति नाम
people-face-ignore = नजरअंदाज करें
people-not-this-person = { $name } नहीं
prefs-views-faces = चेहरे की पहचान
    .subtitle = फोटेमा के शुरू होने पर चेहरे की पहचान सक्षम करें। यह एक समय लेने वाली प्रक्रिया है।
progress-detect-faces-photos = फोटो में चेहरों का पता लगाया जा रहा है।
progress-recognize-faces-photos = फोटो में लोगों को पहचाना जा रहा है।
banner-recognize-faces-photos = फोटो में लोगों को पहचाना जा रहा है। इसमें थोड़ा समय लगेगा।
person-menu-rename = व्यक्ति का नाम बदलें
person-menu-delete = व्यक्ति मिटाएं
person-delete-dialog =
    .heading = व्यक्ति मिटाएं?
    .body = कोई तस्वीर या वीडियो नहीं हटाया जाएगा।
    .cancel-button = रद्द करें
    .delete-button = मिटाएं
person-rename-dialog =
    .heading = व्यक्ति का नाम बदलें?
    .placeholder = नया नाम
    .cancel-button = रद्द करें
    .rename-button = नाम बदलें
people-page-status-off =
    .title = चेहरे की पहचान सक्षम करें?
    .description = { -app-name } स्वचालित रूप से चेहरे का पता लगा सकता है और लोगों को पहचान सकता है, लेकिन यह एक समय लेने वाली प्रक्रिया है। क्या आप इस सुविधा को सक्षम करना चाहते हैं?
    .notice = { -app-name } चेहरे और लोगों को पहचानने के लिए लगभग 45 मेगाबाइट डेटा डाउनलोड करना होगा।
    .enable = सक्षम करें
