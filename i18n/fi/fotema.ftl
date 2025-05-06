# Progress bar for background tasks
# Generating thumbnails from photos
progress-thumbnails-photos = Luodaan kuvan pikkukuvia.
# Notification banner for background tasks
# Processing new videos to extract metadata from video container.
banner-metadata-videos = Käsitellään videon metatietoja.
# Photo/Video Information Sidebar
# Video codec, such as "AV1".
infobar-video-codec = Videokoodekki
# Terms
-app-name = Fotema
# Main Navigation Pages
# Title for library page, which contains the "all", "months", and "years" pages.
library-page = Kirjasto
# Main Navigation Pages
# Title for years album.
years-album = Vuosi
# Main Navigation Pages
# Title for all photos/videos album.
all-album = Päivä
# Main Navigation Pages
# Title for video album.
videos-album = Videot
# Main Navigation Pages
# Title for album showing all folders.
folders-album = Kansiot
# About Dialog
# Section header for open source projects acknowledgements.
about-opensource = Avoimen lähdekoodin projektit
# Photo/Video Viewer
# Go to next button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-next =
    .tooltip = Seuraava
# Photo/Video Viewer
# Tooltip for (i) button to show photo/video information sidebar
viewer-info-tooltip = Näytä ominaisuudet
# Photo/Video Viewer
# Go to previous button when viewing photo or video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-previous =
    .tooltip = Edellinen
# Photo/Video Viewer
# Skip video forwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-forward-10-seconds =
    .tooltip = Kelaa eteenpäin 10 sekuntia
# Photo/Video Viewer
# Mute or unmute audio of a video.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-mute =
    .tooltip = Mykistä/poista mykistys
# Photo/Video Viewer
# Convert all incompatible videos description.
viewer-convert-all-description = Tämä video täytyy muuntaa, ennen kuin se voidaan toistaa. Tämä täytyy tehdä vain kerran, mutta muuntamisessa saattaa kestää hetki.
# Photo/Video Viewer
# Button to convert all incompatible videos.
viewer-convert-all-button = Muunna kaikki yhteensopimattomat videot
# Photo/Video Viewer
# Viewer failed to load an image or video.
viewer-error-failed-to-load = Lataaminen epäonnistui
# Photo/Video Viewer
# Viewer could not display an image or video because it is missing.
# Variables:
#  file_name - (String) path of missing file.
viewer-error-missing-file =
    Tiedostoa ei voi näyttää, koska se puuttuu:
    { $file_name }
# Photo/Video Viewer
# Viewer could not display a file because database entry doesn't have file path.
# If this situation occurs, then I've mucked up the SQL view query and a bug should
# be raised.
viewer-error-missing-path = Tiedostopolku ei ole läsnä tietokannassa
# Photo/Video Information Sidebar
# Name of containing folder of photo or video being viewed.
# Attributes:
#  .tooltip - tooltip text for open folder action button.
infobar-folder = Kansio
    .tooltip = Avaa sisältävä kansio
# Photo/Video Information Sidebar
# File name of photo or video
infobar-file-name = Tiedostonimi
# Photo/Video Information Sidebar
# File creation timestamp from file system metadata.
infobar-file-created = Tiedosto luotu
# Photo/Video Information Sidebar
# File modification timestamp from file system metadata.
infobar-file-modified = Tiedostoa muokattu
# Photo/Video Information Sidebar
# File size file system metadata.
infobar-file-size = Tiedoston koko
# Photo/Video Information Sidebar
# File format, such as "JPEG" or "PNG".
infobar-file-format = Muoto
# Photo/Video Information Sidebar
# File creation timestamp from image or video embedded metadata.
infobar-originally-created = Luoto alun perin
# Photo/Video Information Sidebar
# Duration (HH:MM) of video.
infobar-video-duration = Kesto
# Preferences
# Title of section of preferences for views
prefs-views-section = Näkymät
    .description = Näytä tai piilota sivupalkkinäkymät
# Progress bar for background tasks
# Transcoding videos to a compatible format
progress-convert-videos = Muunnetaan videoita.
# Notification banner for background tasks
# Scanning file system for new videos
banner-scan-videos = Etsitään videoita tiedostojärjestelmästä.
# Notification banner for background tasks
# Updating the database to remove details of absent photos.
banner-clean-photos = Kuvatietokannan ylläpito.
# Main Navigation Pages
# Title for months album.
months-album = Kuukausi
# Notification banner for background tasks
# Extracting video component from Android motion photos
banner-extract-motion-photos = Käsitellään liikekuvia.
# Primary menu
# Menu item to show preferences dialog
primary-menu-preferences = Asetukset
# Primary menu
# Menu item to show "about" dialog
primary-menu-about = Tietoja - { -app-name }
# Main Navigation Pages
# Title for album of selfies.
selfies-album = Selfiet
# Main Navigation Pages
# Title for album of iOS live photos and Android motion photos.
animated-album = Animoitu
# Main Navigation Pages
# Title for album showing contents of one folder.
folder-album = Kansio
# Main Navigation Pages
# Title for places page which shows photos overlayed onto a map.
places-page = Paikat
# Photo/Video Viewer
# Play or pause a video button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-play =
    .tooltip = Toista/keskeytä
# Thumbnail decorations
# Label on month album thumbnails.
# Variables:
#   $month - month number (1 through 12).
#   $year - year e.g., 2024
# Translator note: do not values in square brackets, such as '[other]'.
month-thumbnail-label =
    { $month ->
        [1] Tammikuu { $year }
        [2] Helmikuu { $year }
        [3] Maaliskuu { $year }
        [4] Huhtikuu { $year }
        [5] Toukokuu { $year }
        [6] Kesäkuu { $year }
        [7] Heinäkuu { $year }
        [8] Elokuu { $year }
        [9] Syyskuu { $year }
        [10] Lokakuu { $year }
        [11] Marraskuu { $year }
        [12] Joulukuu { $year }
       *[other] { $year }
    }
# About Dialog
# Translator note: add one translator per-line to get a translation
# credit in the Fotema's "About" page.
about-translator-credits = Jiri Grönroos <jiri.gronroos+l10n@iki.fi>
# Photo/Video Viewer
# Skip video backwards 10 seconds button.
# Attributes:
#  .tooltip - Tooltip on mouse hover.
viewer-skip-backwards-10-seconds =
    .tooltip = Kelaa taaksepäin 10 sekuntia
# Photo/Video Information Sidebar
# Audio codec, such as "OPUS".
infobar-audio-codec = Äänikoodekki
# Notification banner for background tasks
# Processing new photos to extract metadata from EXIF tags.
banner-metadata-photos = Käsitellään kuvan metatietoja.
# Photo/Video Information Sidebar
# Video container format, such as "MKV" or "QuickTime".
infobar-video-container-format = Säilön muoto
# Photo/Video Information Sidebar
# Width and height of photo or video.
infobar-dimensions = Mitat
# Preferences
# Title of preferences dialog
prefs-title = Asetukset
# Photo/Video Information Sidebar
# File modification timestamp from image or video embedded metadata.
infobar-originally-modified = Muokattu alun perin
# Progress bar for background tasks
# Extracting details from photo EXIF data
progress-metadata-photos = Käsitellään kuvan metatietoja.
# Preferences
# Selfies page enabled or disabled.
# Attributes:
#   .subtitle - Description of toggle button action action.
prefs-views-selfies = Selfiet
    .subtitle = Näyttää erillisen näkymän iOS-laitteilla otetuista selfieistä. Käynnistä { -app-name } uudelleen, jotta muutos tulee voimaan.
# Progress bar for background tasks
# Extracting details from video container metadata
progress-metadata-videos = Käsitellään videon metatietoja.
# Progress bar for background tasks
# Generating thumbnails from videos
progress-thumbnails-videos = Luodaan videon pikkukuvia.
# Progress bar for background tasks
# Extracting motion photo videos
progress-motion-photo = Käsitellään liikekuvia.
# Notification banner for background tasks
# Scanning file system for new photos
banner-scan-photos = Etsitään kuvia tiedostojärjestelmästä.
# Progress bar for background tasks
# Not doing any background work
progress-idle = Jouten.
# Notification banner for background tasks
# Generating thumbnails for all photos.
banner-thumbnails-photos = Luodaan kuvan pikkukuvia. Tämä kestää hetken.
# Notification banner for background tasks
# Generating thumbnails for all videos.
banner-thumbnails-videos = Luodaan videon pikkukuvia. Tämä kestää hetken.
# Notification banner for background tasks
# Updating the database to remove details of absent videos.
banner-clean-videos = Videotietokannan ylläpito.
people-face-ignore = Ohita
people-not-this-person = Ei { $name }
progress-detect-faces-photos = Havaitaan kasvoja kuvissa.
progress-recognize-faces-photos = Tunnistetaan ihmisiä kuvissa.
person-menu-delete = Poista henkilö
person-menu-rename = Nimeä henkilö uudelleen
people-page-status-off =
    .title = Otetaanko kasvojentunnistus käyttöön?
    .description = { -app-name } voi havaita kasvot ja tunnistaa ihmisiä automaattisesti, mutta siinä voi kestää kauan. Haluatko käyttää tätä ominaisuutta?
    .notice = { -app-name }n tulee ladata noin 45 megatavua dataa kasvojen ja ihmisten tunnistamiseksi.
    .enable = Ota käyttöön
people-page = Ihmiset
people-set-face-thumbnail = Käytä pienoiskuvana
people-set-name = Aseta nimi
people-page-status-no-people =
    .title = Ihmisiä ei löytynyt
    .description =
        { -app-name } etsii kasvoja uusista kuvista, kun sovellus käynnistyy.
        Anna ihmisille nimi, jotta { -app-name } voi tehdä albumin jokaisesta henkilöstä.
viewer-faces-menu =
    .tooltip = Kasvot-valikko
    .restore-ignored = Palauta kaikki ohitetut kasvot
    .ignore-unknown = Ohita kaikki tuntemattomat kasvot
    .scan = Tunnista lisää kasvoja
people-person-search =
    .placeholder = Henkilön nimi
banner-detect-faces-photos = Havaitaan kasvoja kuvissa. Tämä kestää hetken.
banner-recognize-faces-photos = Tunnistetaan ihmisiä kuvissa. Tämä kestää hetken.
prefs-views-faces = Kasvojentunnistus
    .subtitle = Ota kasvojentunnistus käyttöön, kun Fotema käynnistyy. Kasvojentunnistuksessa voi kestää kauan.
person-delete-dialog =
    .heading = Poistetaanko henkilö?
    .body = Kuvia tai videoita ei poisteta.
    .cancel-button = Peru
    .delete-button = Poista
person-rename-dialog =
    .heading = Nimetäänkö henkilö uudelleen?
    .placeholder = Uusi nimi
    .cancel-button = Peru
    .rename-button = Nimeä uudelleen
prefs-machine-learning-section = Koneoppiminen
    .description = Määritä koneoppimisen ominaisuudet.
prefs-machine-learning-face-detection = Kasvojentunnistus
    .subtitle = Käytä kasvojentunnistusta, kun { -app-name } käynnistyy. Tämä toimenpide saattaa kestää kauan.
prefs-ui-section = Käyttöliittymä
    .description = Muokkaa käyttöliittymää.
prefs-ui-chronological-album-sort = Järjestys
    .subtitle = Aikaperusteinen järjestys albumeille.
    .ascending = Nouseva
    .descending = Laskeva
prefs-ui-selfies = Selfiet
    .subtitle = Näyttää erillisen albumin iOS-laitteilla otetuista selfieistä. Käynnistä { -app-name } uudelleen, jotta asetus tulee voimaan.
banner-button-stop =
    .label = Pysäytä
    .tooltip = Pysäytä kaikki taustatehtävät.
banner-stopping = Pysätetään tehtäviä...
banner-convert-videos = Muunnetaan videoita.
prefs-library-section-pictures-dir =
    .title = Kuvat-kansio
    .tooltip = Valitse kuvien kansio.
prefs-library-section =
    .title = Kirjasto
    .description =
        Määritä kirjaston kansio.
        Varoitus: kuvakansion vaihtaminen voi aiheuttaa sen, että { -app-name } käsittelee kaikki kuvat uudelleen.
onboard-select-pictures =
    .title = Tervetuloa, tämä on { -app-name }.
    .description =
        Valitse kansio, jossa pidät kuvakirjastoasi.

        Jos olet käyttänyt { -app-name }n aiempia versioita, joissa kuvakirjasto havaittiin automaattisesti, niin valitse sama kansio välttääksesi kuvien käsittelyn uudelleen.
    .button = Valitse kansio
prefs-albums-chronological-sort = Järjestys
    .subtitle = Aikaan perustuva järjestys albumeille.
    .ascending = Nousevasti
    .descending = Laskevasti
prefs-processing-section = Kuvan- ja videonkäsittely
    .description = Määritä kuvan- ja videokäsittelyn ominaisuudet.
prefs-processing-face-detection = Kasvojentunnistus
    .subtitle = Havaitse kasvoja ja tunnista nimeämiäsi ihmisiä. Tämä on aikaavievä toimenpide.
prefs-processing-motion-photos = Liikekuvat
    .subtitle = Havaitse Androidin liikekuvat ja pura videoita.
banner-scan-library = Läpikäydään kirjastoa.
prefs-albums-section = Albumit
    .description = Muokkaa albumeja.
prefs-albums-selfies = Selfies
    .subtitle = Näyttää erillisen albumin iOS-laitteilla otetuista selfiekuvista. Käynnistä { -app-name } uudelleen, jotta muutokset tulevat voimaan.
