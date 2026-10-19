# Razor conventions

Use semantic HTML with the shared `platform-*` classes. Loading content uses `role="status"`,
errors use `role="alert"`, unavailable content uses explicit text rather than color alone, and
forms pair labels with controls and `aria-describedby` messages. Admin navigation is rendered
from application-owned permission metadata; these client hints never replace endpoint policy
enforcement.
