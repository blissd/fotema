<!--
SPDX-FileCopyrightText: © 2024 David Bliss

SPDX-License-Identifier: GFDL-1.3-or-later
-->
# Localisation

Fotema is internationalised with the [Fluent](https://projectfluent.org/)
i18n framework.

Localizations are found under the `i18n` directory.

## Add A New Localisation

The default language is US English, and the localisation file is found at
`i18n/en-US/fotema.ftl`.

To add a new language localisation, create a directory with the appropriate
language code, such as `i18n/fr` and copy the contents of `i18n/en-US` to
bootstrap your translation.

All localisations must be licensed as GPL 3.0 or later.

## Local Testing

When Fotema builds it checks that message identifiers exist, but _only_
for the default locale (en-US). So to test your locale has all message
identifiers you can temporarily change the default local by editing the
`fallback_language` in the `i18n.toml` file and then running a devel build.
Any missing or invalid message identifier will show up as compilation errors:

```shell
$ sed -i 's/en-US/fr/' i18n.toml
$ just devel
```

## Running Flatpak With A Different Locale

If you have added a new localization, then you can produce a test build
of Fotema and run with:

```shell
just devel
LANG=fr flatpak run app.fotema.Fotema
```

