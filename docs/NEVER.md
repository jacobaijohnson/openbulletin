# The never list (public on purpose)

Features OpenBulletin deliberately will not build, with the reasoning a
small volunteer project can defend:

- **Mail merge / send-as-email** — email is the email client's job; it
  drags in SMTP, contacts, and privacy liability.
- **Web publishing / HTML export / EPUB / interactive PDF** — Microsoft
  itself removed web features from Publisher. Websites are another product.
  *Amended 2026-10-16: HTML export shipped on Jacob's directive of
  2026-10-15 (docs/planning/beyond-print.md): one self-contained web
  page carrying the bulletin's words as a reading edition, made to be
  read and searched on a screen. Web publishing as a product, EPUB,
  and interactive PDF remain never.*
- **GREP styles, nested styles, conditional text** — pro-typesetter power
  with a huge edge-case surface; styles cover the real need.
- **Footnotes, endnotes, indexes, book files** — long-document territory,
  already well served elsewhere.
- **Real-time collaboration / cloud sync** — files on a shared drive work;
  we will not run servers for churches.
- **Bundled stock photos / clip art** — licensing liability we cannot police.
  *Amended 2026-10-16: a starter set of clip art shipped on Jacob's
  directive of 2026-10-15 (docs/research/clipart-landscape.md): a
  small set of line-art pieces, each one checked by hand against a
  public-domain or CC0 source and recorded in a manifest with its
  author and license. Bundled stock photos, and any art that cannot
  be verified piece by piece, remain never.*
- **Bundled hymn lyrics or modern Bible translations** — copyrighted
  (CCLI, NRSV, ESV). We ship public-domain texts and reading *references*.
- **AI writing/generation in the app** — no server, no cost, no privacy
  questions. Paste from whatever tool you like.
- **`.pub` export, `.indd` reading** — undocumented proprietary binaries;
  writing `.pub` would extend a dead format.
- **Full PDF editing** — PDF is an output format. "Edit any PDF" is a
  separate, bottomless product.
- **Scripting / plugin marketplace** — deferred until the API can be kept
  stable; a CLI ships first.
