## Terms


# See https://projectfluent.org/fluent/guide/terms.html

-app-name = Fotema

## Main Navigation Pages

# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Mediathek
# Title for years album.
years-album = Jahr
# Title for months album.
months-album = Monat
# Title for all photos/videos album.
all-album = Tag
# Title for video album.
videos-album = Videos
# Title for album of selfies.
selfies-album = Selfies
# Title for album of iOS live photos and Android motion photos.
animated-album = Animiert
# Title for album showing all folders.
folders-album = Ordner
# Title for album showing contents of one folder.
folder-album = Ordner
# Title for places page which shows photos overlayed onto a map.
places-page = Orte

## Thumbnail decorations

# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] Januar { $year }
        [2] Februar { $year }
        [3] März { $year }
        [4] April { $year }
        [5] Mai { $year }
        [6] Juni { $year }
        [7] Juli { $year }
        [8] August { $year }
        [9] September { $year }
        [10] Oktober { $year }
        [11] November { $year }
        [12] Dezember { $year }
       *[other] { $year }
    }

## About Dialog

# Section header for open source projects acknowledgements.
about-opensource = Quelloffene Projekte
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = David Bliss <hello@fotema.app>

## Photo/Video Viewer

# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Eigenschaften anzeigen
# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
    .tooltip = Weiter
# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
    .tooltip = Zurück
# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
    .tooltip = Fortsetzen/Pausieren
# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
    .tooltip = 10 Sekunden vorwärts springen
# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
    .tooltip = 10 Sekunden rückwärts springen
# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
    .tooltip = Stumm/Laut
# Convert all incompatible videos description.
viewer-convert-all-description = Dieses Video muss konvertiert werden, bevor es abgespielt werden kann. Dies muss nur einmal geschehen, aber es dauert, das Video zu konvertieren.
# Button to convert all incompatible videos.
viewer-convert-all-button = Alle inkompatiblen Videos konvertieren
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Laden fehlgeschlagen
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Die Datei kann nicht angezeigt werden, weil sie fehlt:
    { $file_name }
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Dateipfad nicht in der Datenbank vorhanden

## Photo/Video Information Sidebar

# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Ordner
    .tooltip = Beinhaltenden Ordner öffnen
# File name of photo or video
infobar-file-name = Dateiname
# File creation timestamp from file system metadata.
infobar-file-created = Datei erstellt
# File modification timestamp from file system metadata.
infobar-file-modified = Datei verändert
# File size file system metadata.
infobar-file-size = Dateigröße
# File format, such as "JPEG" or "PNG".
infobar-file-format = Bildformat
# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Original erstellt
# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Original verändert
# Duration (HH:MM) of video.
infobar-video-duration = Dauer
# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Container-Format
# Video codec, such as "AV1".
infobar-video-codec = Video Codec
# Audio codec, such as "OPUS".
infobar-audio-codec = Audio Codec
# Width and height of photo or video.
infobar-dimensions = Bildgröße

## Preferences

# Title of preferences dialog
prefs-title = Einstellungen
# Title of section of preferences for views
prefs-views-section = Ansicht
    .description = Seitenleiste anzeigen/verstecken
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfies
    .subtitle = Zeige eine separate Ansicht für Selfies, die mit iOS-Geräten aufgenommen wurden. Starte { -app-name } neu, um es anzuwenden.

## Progress bar for background tasks

# Extracting details from photo EXIF data
progress-metadata-photos = Foto-Metadaten werden verarbeitet.
# Extracting details from video container metadata
progress-metadata-videos = Video-Metadaten werden verarbeitet.
# Generating thumbnails from photos
progress-thumbnails-photos = Foto-Miniaturansichten werden erstellt.
# Generating thumbnails from videos
progress-thumbnails-videos = Video-Miniaturansichten werden erstellt.
# Transcoding videos to a compatible format
progress-convert-videos = Videos werden konvertiert.
# Extracting motion photo videos
progress-motion-photo = Bewegungsfotos werden verarbeitet.
# Not doing any background work
progress-idle = Nichts zu tun.

## Notification banner for background tasks


# Similar to the progress bar, but allows for longer messages.

# Scanning file system for new photos
banner-scan-photos = Dateien werden nach Fotos durchsucht.
# Scanning file system for new videos
banner-scan-videos = Dateien werden nach Videos durchsucht.
# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Foto-Metadaten werden verarbeitet.
# Processing new videos to extract metadata from video container.
banner-metadata-videos = Video-Metadaten werden verarbeitet.
# Generating thumbnails for all photos.
banner-thumbnails-photos = Foto-Miniaturansichten werden erstellt. Dies kann eine Weile dauern.
# Generating thumbnails for all videos.
banner-thumbnails-videos = Video-Miniaturansichten werden erstellt. Dies kann eine Weile dauern.
# Updating the database to remove details of absent photos.
banner-clean-photos = Fotodatenbank wird optimiert
# Updating the database to remove details of absent videos.
banner-clean-videos = Videodatenbank wird optimiert
# Extracting video component from Android motion photos
banner-extract-motion-photos = Bewegungsfotos werden verarbeitet

## Primary menu


# The "hamburger" menu on the main app navigation sidebar.

# Menu item to show preferences dialog
primary-menu-preferences = Einstellungen
# Menu item to show "about" dialog
primary-menu-about = Info zu { -app-name }
people-person-search =
    .placeholder = Name der Person
people-face-ignore = Ignorieren
people-not-this-person = Nicht { $name }
prefs-views-faces = Gesichtserkennung
    .subtitle = Aktivieren Sie die Gesichtserkennung beim Starten von Fotema. Dies kann eine Weile dauern.
person-rename-dialog =
    .heading = Person umbenennen?
    .placeholder = Neuer Name
    .cancel-button = Abbrechen
    .rename-button = Umbenennen
people-page-status-no-people =
    .title = Keine Personen gefunden
    .description =
        { -app-name } wird nach dem Start nach Gesichtern in neuen Fotos suchen.
        Benennen Sie die Personen auf Ihren Fotos, damit { -app-name } für jede Person ein Album erstellen kann.
viewer-faces-menu =
    .tooltip = Gesichter-Menü
    .restore-ignored = Alle ignorierten Gesichter wiederherstellen
    .ignore-unknown = Alle unbekannten Gesichter ignorieren
    .scan = Nach weiteren Gesichtern suchen
people-page = Personen
people-page-status-off =
    .title = Gesichtserkennung aktivieren?
    .description = { -app-name } kann automatisch Gesichter und Personen erkennen, jedoch ist dies ein zeitaufwändiger Prozess. Möchten Sie diese Funktion aktivieren?
    .notice = { -app-name } muss etwa 45 Megabyte an Daten herunterladen, um Gesichter und Personen zu erkennen.
    .enable = Aktivieren
people-set-face-thumbnail = Als Vorschaubild verwenden
people-set-name = Name festlegen
progress-detect-faces-photos = Erkennung von Gesichtern auf Fotos.
progress-recognize-faces-photos = Erkennen von Personen auf Fotos.
banner-detect-faces-photos = Erkennen von Gesichtern auf Fotos. Dies wird eine Weile dauern.
banner-recognize-faces-photos = Erkennen von Personen auf Fotos. Dies wird eine Weile dauern.
person-menu-rename = Person umbenennen
person-menu-delete = Person löschen
person-delete-dialog =
    .heading = Person löschen?
    .body = Es werden keine Bilder oder Videos gelöscht.
    .cancel-button = Abbrechen
    .delete-button = Löschen
prefs-ui-selfies = Selfies
    .subtitle = Zeigt ein separates Album für Selfies an, die mit iOS-Geräten aufgenommen wurden. Starten Sie { -app-name } neu, um es anzuwenden.
prefs-ui-chronological-album-sort = Sortierreihenfolge
    .subtitle = Chronologische Sortierreihenfolge für Alben.
    .ascending = Aufsteigend
    .descending = Absteigend
prefs-machine-learning-face-detection = Gesichtserkennung
    .subtitle = Aktivieren Sie die Gesichtserkennung beim Starten von { -app-name }. Dies ist ein zeitaufwändiger Prozess.
prefs-ui-section = UI
    .description = Optimieren Sie die Benutzeroberfläche.
prefs-library-section =
    .title = Mediathek
    .description =
        Konfiguriere die Mediathek.
        Warnung: Ein Wechsel des Bilderverzeichnisses kann dazu führen, dass { -app-name } alle Ihre Bilder neu verarbeitet.
prefs-library-section-pictures-dir =
    .title = Bilder-Verzeichnis
    .tooltip = Wähle das Verzeichnis der Bilder.
banner-button-stop =
    .label = Stop
    .tooltip = Beende alle Prozesse im Hintergrund.
onboard-select-pictures =
    .title = Willkommen bei { -app-name }.
    .description =
        Bitte wähle das Verzeichnis aus, in dem deine Bilder aufbewahrt werden.

        Wenn du eine frühere Version von { -app-name } verwendet hast, in dem das Bild-Verzeichnis automatisch erkannt wurde, wähle hier bitte das gleiche Verzeichnis aus, um eine doppelte Verarbeitung von Bildern zu vermeiden.
    .button = Ordner auswählen
prefs-machine-learning-section = Künstliches Lernen
    .description = Konfiguriere die Funktionen für künstliches Lernen.
banner-stopping = Prozesse werden angehalten...
banner-convert-videos = Videos werden umgewandelt.
