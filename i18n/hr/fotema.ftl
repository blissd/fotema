-app-name = Fotema
library-page = Biblioteka
years-album = Godina
months-album = Mjesec
all-album = Dan
videos-album = Videa
selfies-album = Selfiji
animated-album = Animirano
folders-album = Mape
folder-album = Mapa
places-page = Mjesta
people-page = Osobe
people-page-status-off =
    .title = Aktivirati otkrivanje lica?
    .description = { -app-name } može automatski otkriti lica i prepoznati osobe, ali to je dugovlačan proces. Želiš li aktivirati ovu funkciju?
    .notice = Aplikacija { -app-name } mora preuzeti oko 45 megabajta podataka kako bi prepoznala lica i osobe.
    .enable = Aktiviraj
people-page-status-no-people =
    .title = Nema pronađenih ljudi
    .description =
        { -app-name } će tražiti lica u novim fotografijama tijekom pokretanja.
        Imenuj osobe u fotografijama kako bi { -app-name } mogao stvoriti album za svaku osobu.
month-thumbnail-label =
    { $month ->
        [1] Siječanj { $year }.
        [2] Veljača { $year }.
        [3] Ožujak { $year }.
        [4] Travanj { $year }.
        [5] May { $year }.
        [6] Lipanj { $year }.
        [7] Srpanj { $year }.
        [8] Kolovoz { $year }.
        [9] Rujan { $year }.
        [10] Listopad { $year }.
        [11] Studeni { $year }.
        [12] Prosinac { $year }.
       *[other] { $year }.
    }
about-opensource = Projekti otvorenog koda
about-translator-credits = Milo Ivir <mail@milotype.de>
viewer-info-tooltip = Prikaži svojstva
viewer-faces-menu =
    .tooltip = Izbornik lica
    .restore-ignored = Obnovi zanemarena lica
    .ignore-unknown = Zanemari sva nepoznata lica
    .scan = Pretraži daljnja lica
viewer-next =
    .tooltip = Sljedeće
viewer-previous =
    .tooltip = Prethodno
viewer-play =
    .tooltip = Pokreni/Pauza
viewer-skip-forward-10-seconds =
    .tooltip = Preskoči 10 sekundi naprijed
viewer-skip-backwards-10-seconds =
    .tooltip = Preskoči 10 sekundi natrag
viewer-mute =
    .tooltip = Isključi/Uključi zvuk
viewer-convert-all-description = Ovaj se video mora konvertirati prije nego što se može reproducirati. To bi se trebalo dogoditi samo jednom, ali konvertiranje može potrajati.
viewer-convert-all-button = Konvertiraj sva nekompatibilna videa
viewer-error-failed-to-load = Neuspjelo učitavanje
viewer-error-missing-file =
    Nije moguće prikazati datoteku jer nedostaje:
    { $file_name }
viewer-error-missing-path = Staza datoteke ne postoji u bazi podataka
infobar-folder = Mapa
    .tooltip = Otvori mapu fotografija i videa
infobar-file-name = Ime datoteke
infobar-file-created = Datoteka stvorena
infobar-file-modified = Datoteka promijenjena
infobar-file-size = Veličina datoteke
infobar-file-format = Format
infobar-originally-created = Izvorno stvoreno
infobar-originally-modified = Izvorno promijenjeno
infobar-video-duration = Trajanje
infobar-video-container-format = Format kontejnera
infobar-video-codec = Video kodek
infobar-audio-codec = Audio kodek
infobar-dimensions = Dimenzije
people-set-face-thumbnail = Koristi kao minijaturu
people-set-name = Postavi ime
people-person-search =
    .placeholder = Ime osobe
people-face-ignore = Zanemari
people-not-this-person = Nije { $name }
prefs-title = Postavke
prefs-albums-section = Albumi
    .description = Konfiguriraj albume.
prefs-albums-selfies = Selfiji
    .subtitle = Prikazuje zaseban album za selfije snimljene na iOS uređajima. Za primjenu ponovo pokreni { -app-name }.
prefs-albums-chronological-sort = Redoslijed
    .subtitle = Kronološki redoslijed za albume.
    .ascending = Uzlazno
    .descending = Silazno
prefs-processing-section = Obrada fotografija i videa
    .description = Konfiguriraj funkcije obrade fotografija i videa.
prefs-processing-face-detection = Otkrivanje lica
    .subtitle = Otkrij lica i prepoznaj osobe koje si imenovao/la. To je dugotrajan proces.
prefs-processing-motion-photos = Motion fotografije
    .subtitle = Otkrivanje Android „motion“ fotografija i izdvajanje videa.
prefs-library-section =
    .title = Biblioteka
    .description =
        Konfiguriraj direktorij biblioteke.
        Upozorenje: mijenjanje direktorija slika može prouzročiti da { -app-name } ponovo obradi sve tvoje slike.
prefs-library-section-pictures-dir =
    .title = Direktorij slika
    .tooltip = Odaberi direktorij slika.
progress-metadata-photos = Obrada metapodataka fotografija.
progress-metadata-videos = Obrada metapodataka videa.
progress-thumbnails-photos = Generiranje minijatura fotografija.
progress-thumbnails-videos = Generiranje minijatura videa.
progress-thumbnails-faces = Generiranje minijatura lica.
progress-convert-videos = Konvertiranje videa.
progress-motion-photo = Obrađivanje „motion“ fotografija.
progress-detect-faces-photos = Otkrivanje lica u fotografijama.
progress-recognize-faces-photos = Prepoznavanje osoba u fotografijama.
progress-idle = Bez aktivnosti.
banner-scan-library = Pretraživanje biblioteke.
banner-metadata-photos = Obrada metapodataka fotografija.
banner-metadata-videos = Obrada metapodataka videa.
banner-thumbnails-photos = Generiranje minijatura fotografija. Ovo će potrajati.
banner-thumbnails-videos = Generiranje minijatura videa. Ovo će potrajati.
banner-clean-photos = Održavanje baze podataka fotografija.
banner-clean-videos = Održavanje baze podataka videa.
banner-extract-motion-photos = Obrađivanje „motion“ fotografija.
banner-detect-faces-photos = Otkrivanje osoba u fotografijama. Ovo će potrajati.
banner-recognize-faces-photos = Prepoznavanje osoba u fotografijama. Ovo će potrajati.
banner-convert-videos = Konvertiranje videa.
banner-face-thumbnails = Generiranje minijatura lica
banner-button-stop =
    .label = Prekini
    .tooltip = Prekini sve pozadinske zadatke.
banner-stopping = Prekidanje zadataka …
primary-menu-preferences = Postavke
primary-menu-about = O aplikaciji { -app-name }
person-menu-rename = Preimenuj osobu
person-menu-delete = Izbriši osobu
person-delete-dialog =
    .heading = Izbrisati osobu?
    .body = Neće se izbrisati nijedna slika ili video.
    .cancel-button = Odustani
    .delete-button = Izbriši
person-rename-dialog =
    .heading = Preimenovati osobu?
    .placeholder = Novo ime
    .cancel-button = Odustani
    .rename-button = Preimenuj
onboard-select-pictures =
    .title = Dobro došli u { -app-name }.
    .description =
        Odaberi direktorij u kojem čuvaš svoju biblioteku slika.

        Ako si koristio/la raniju { -app-name } verziju gdje je tvoja biblioteka slika automatski otkrivena, odaberi isti direktorij ovdje za izbjegavanje duplicirannja obrade slika.
    .button = Odaberi direktorij
