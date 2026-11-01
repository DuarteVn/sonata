# Feature Specification: Report Bug

**Feature Branch**: `013-report-bug`

**Created**: 2026-11-01

**Status**: Draft

**Input**: User description: "Feature 013: Report Bug. Users can report a bug from the app: Help menu "Report Bug…" and a shortcut in the sidebar footer open a dialog. The dialog shows, prominently, the playful message in Portuguese "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA" (this message stays in Portuguese; rest of the UI stays English). Fields: reporter name, bug description, optional image (PNG/JPG up to 5 MB, via file picker, paste with Cmd/Ctrl+V, or drag and drop). Diagnostics attached automatically: app version, OS, architecture. Logs attached only if the user ticks a checkbox. On send, the app POSTs to a small Cloudflare Worker (code lives in this repo under a worker folder) that emails the report with the image attached via Resend to the maintainer's email; the Resend API key and destination email live only in Worker secrets, never in the repo or the app. Worker has rate limiting and size validation against abuse. After send, show confirmation; on failure, offer a mailto: fallback with the text prefilled (no attachment). Works on macOS, Linux, Windows."

## Clarifications

### Session 2026-11-01

- Q: The email fallback needs an address built into the app, visible to every user. Which one? → A: A separate public alias created by the maintainer; the address the service delivers to stays secret. Like the service address, the alias is set at build time for releases; a build without it offers only "copy the report text".

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Friend sends a bug report with a screenshot (Priority: P1)

A friend using Sonata hits a problem. They open "Report Bug…" from the Help menu, smile at
the message "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA", type their name and what
went wrong, paste a screenshot, and press Send. A few seconds later the maintainer receives
an email with the description, the screenshot attached, and the app version and operating
system of the friend's machine.

**Why this priority**: This is the whole feature. Today friends report problems through
scattered chat messages without version or system details, and screenshots get lost.

**Independent Test**: With a report service deployed for testing, open the dialog, fill
name and description, attach a PNG, send, and check that one email arrives at the configured
address with the text, the image, and the diagnostics.

**Acceptance Scenarios**:

1. **Given** Sonata is open, **When** the user chooses Help → "Report Bug…", **Then** the
   report dialog opens and shows the message "Nosso sistema não tem bugs, tem PONTOS DE
   MELHORIA" as its most prominent text.
2. **Given** the dialog is open, **When** the user fills name and description and presses
   Send, **Then** the dialog shows a sending state, and then a confirmation that the report
   was sent.
3. **Given** the user attached an image, **When** the report is delivered, **Then** the email
   contains the image as an attachment that opens in a normal image viewer.
4. **Given** any report, **When** it is delivered, **Then** the email includes the app
   version, the operating system with its version, and the CPU architecture, without the
   user typing them.
5. **Given** the name or the description is empty, **When** the user looks at the Send
   button, **Then** it is disabled and the empty required field is marked.

---

### User Story 2 - Attach an image in the most natural way (Priority: P2)

The user already has a screenshot in the clipboard, or as a file on the desktop. They paste it
with Cmd+V (Ctrl+V on Linux and Windows), drag the file onto the dialog, or pick it with a file
chooser. They see a preview and can remove it before sending.

**Why this priority**: A picture explains most canvas and terminal problems faster than
text. Paste is the common path because the OS screenshot tools copy to the clipboard.

**Independent Test**: Open the dialog and attach an image three ways (paste, drag and drop,
file chooser); each time a preview appears, and removing it clears the attachment.

**Acceptance Scenarios**:

1. **Given** a PNG image in the clipboard, **When** the user pastes into the dialog outside a
   text field, or into the description field, **Then** the image becomes the attachment and a
   preview is shown; text in the clipboard pastes as text as usual.
2. **Given** a JPG file on disk, **When** the user drags it onto the dialog, **Then** it
   becomes the attachment with a preview.
3. **Given** the user presses "Attach image…", **When** they pick a PNG or JPG file, **Then**
   it becomes the attachment with a preview.
4. **Given** an attachment exists, **When** the user adds another image, **Then** the new image
   replaces the old one (one image per report).
5. **Given** an image larger than 5 MB or of another type, **When** the user tries to attach
   it, **Then** it is rejected with a message that names the limit, and any earlier attachment
   is kept.
6. **Given** an attachment exists, **When** the user presses remove on the preview, **Then**
   the report has no image.

---

### User Story 3 - Send logs when asked (Priority: P2)

For problems that are hard to describe, the user ticks "Include logs". The report then carries
Sonata's recent log so the maintainer can see what happened. Without the tick, no log leaves
the machine.

**Why this priority**: Logs speed up debugging but can contain folder paths and project
names, so they must be the user's choice.

**Independent Test**: Send one report with the box unticked and one with it ticked; only the
second email has a log attachment, and it contains only recent Sonata log lines.

**Acceptance Scenarios**:

1. **Given** the dialog opens, **When** the user looks at "Include logs", **Then** it is
   unticked, with a short note that logs can contain folder paths and project names.
2. **Given** the box is ticked, **When** the report is delivered, **Then** the email has a text
   attachment with the most recent part of Sonata's log, at most 1 MB.
3. **Given** the box is unticked, **When** the report is delivered, **Then** no log content is
   included anywhere in the email.

---

### User Story 4 - The report still reaches the maintainer when sending fails (Priority: P3)

The user is offline, the report service is down, or it refuses the report. Sonata says so
plainly, keeps everything the user typed, and offers to open an email to the maintainer in the
user's own mail app, prefilled with the text and diagnostics.

**Why this priority**: Failures are rare, but losing a typed report is frustrating, and the
fallback costs little.

**Independent Test**: Point the app at an unreachable service, send a report, check the error
and that the form keeps its content; press the fallback and check that the mail app opens with
the subject, the description, and the diagnostics filled in.

**Acceptance Scenarios**:

1. **Given** no network, **When** the user presses Send, **Then** the dialog shows that the
   report could not be sent, keeps name, description, image, and the logs choice, and offers
   "Try again" and "Send by email instead".
2. **Given** the service refuses the report because of the rate limit, **When** the error
   shows, **Then** it tells the user to try again later.
3. **Given** the user chooses "Send by email instead", **When** the mail app opens, **Then**
   the message is addressed to the maintainer, the subject names Sonata and the version, and
   the body has the name, description, and diagnostics; the dialog notes that the image must
   be attached by hand.
4. **Given** no mail app is set up, **When** the fallback cannot open one, **Then** the dialog
   offers to copy the report text to the clipboard.

---

### User Story 5 - Maintainer runs the report service at no cost (Priority: P1)

The maintainer deploys the report service once from this repository, using free plans,
following the README. The secret key for the email provider and the maintainer's own address
are set as service secrets and never appear in the repository, the app, or its releases.

**Why this priority**: Without the service nothing is delivered; the repository is public, so
any secret in it would leak at once.

**Independent Test**: Follow the README on a fresh account, deploy the service, set the two
secrets, send a test report with the documented command, receive the email; search the
repository and a built app for the key and the address and find neither.

**Acceptance Scenarios**:

1. **Given** the README section, **When** the maintainer follows it, **Then** the service is
   deployed and delivers a test report, with no paid plan.
2. **Given** a built release, **When** anyone inspects it, **Then** it contains the service
   address only, never the email provider key or the destination address.
3. **Given** more reports from one network address than the limit allows, **When** another
   arrives, **Then** the service refuses it without sending email.
4. **Given** a request larger than the size limit, with a non-image attachment, or with
   missing fields, **When** it arrives, **Then** the service refuses it without sending email.

---

### Edge Cases

- The user closes the dialog with content typed: the draft is kept until the app quits, so
  reopening the dialog shows it again; after a successful send the form is cleared.
- The user presses Send twice quickly: only one report is sent.
- The description is very long: it is limited to 10,000 characters, with a visible counter
  near the limit.
- The name has non-Latin letters or emoji: it is delivered unchanged.
- The pasted image is huge in pixels but under 5 MB: it is sent as is.
- The log is missing or cannot be read: the report is still sent, and the email says logs were
  requested but unavailable.
- Sending takes long: after 30 seconds the attempt counts as failed and the failure flow
  starts.
- The app has no report service address configured (for example a build from source without
  one): Send goes straight to the email fallback, and the dialog says why.
- Someone calls the service directly, not from the app: the same validation and rate limit
  apply; the service accepts only the documented report format.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST have a Help menu on macOS, Linux, and Windows with a "Report Bug…"
  item that opens the report dialog.
- **FR-002**: The sidebar footer MUST have a control that opens the same dialog.
- **FR-003**: The dialog MUST show the message "Nosso sistema não tem bugs, tem PONTOS DE
  MELHORIA" exactly as written, as its most prominent text. This message stays in Portuguese
  in every build; all other dialog text is English.
- **FR-004**: The dialog MUST have a required name field (up to 100 characters), a required
  description field (up to 10,000 characters), an optional single image attachment, and an
  "Include logs" option that starts unticked.
- **FR-005**: Users MUST be able to attach the image by paste, by drag and drop onto the
  dialog, and by a file chooser. Only PNG and JPEG images up to 5 MB are accepted; others are
  rejected with a message naming the limit.
- **FR-006**: The dialog MUST show a preview of the attached image and let the user remove it.
- **FR-007**: Every report MUST include, without user input: app version, operating system
  name and version, CPU architecture, and the date and time of sending.
- **FR-008**: When "Include logs" is ticked, the report MUST include the most recent part of
  Sonata's own log, at most 1 MB; when unticked, no log content is sent.
- **FR-009**: On Send, the app MUST deliver the report to the project's report service and
  show sending, success, and failure states. Send MUST be disabled while sending and while
  required fields are empty.
- **FR-010**: On failure (no network, timeout after 30 seconds, refusal, or no service address
  configured), the app MUST keep the form content and offer "Try again" and "Send by email
  instead".
- **FR-011**: "Send by email instead" MUST open the user's mail app with a message to the
  maintainer's public fallback address (a separate alias, never the address the service
  delivers to), a subject with "Sonata", the version, and the first words of
  the description, and a body with name, description, and diagnostics. When no mail app opens,
  the app MUST offer to copy the same text to the clipboard.
- **FR-012**: The report service MUST email each accepted report to the maintainer's address,
  with the description and diagnostics in the body, the image as an attachment, and the log as
  a text attachment when present. The reporter's name MUST appear in the subject.
- **FR-013**: The report service MUST refuse, without sending email, requests that exceed 7 MB
  in total, carry an attachment that is not PNG or JPEG, miss a required field, or exceed the
  field limits.
- **FR-014**: The report service MUST limit each network address to 5 accepted reports per
  hour and the whole service to 100 per day; requests over the limit are refused with a
  "try later" answer.
- **FR-015**: The email provider key and the destination address MUST exist only as report
  service secrets. They MUST NOT be committed to the repository or built into the app.
- **FR-016**: The report service MUST NOT store reports after the email is handed to the email
  provider.
- **FR-017**: The service source code and its deployment steps MUST live in this repository;
  the README MUST document deployment on free plans, setting the secrets, and a test command.
- **FR-018**: The app MUST send a report only when the user presses Send. Nothing is sent in
  the background.
- **FR-019**: The draft MUST survive closing and reopening the dialog until the app quits, and
  MUST be cleared after a successful send.

### Key Entities

- **Bug Report**: what the user sends. Reporter name, description, optional image, optional
  log excerpt, diagnostics, time of sending.
- **Diagnostics**: app version, operating system and version, CPU architecture.
- **Report Service**: the small web service run by the maintainer that checks a report,
  applies the limits, and emails it. Holds the email provider key and destination address as
  secrets.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can open the dialog, fill it, attach a screenshot, and send a report in
  under 1 minute.
- **SC-002**: 95% of accepted reports reach the maintainer's inbox within 1 minute of Send.
- **SC-003**: Every delivered report carries version, operating system, and architecture
  (100%), so the maintainer never has to ask for them.
- **SC-004**: A search of the repository and of every release file finds neither the email
  provider key nor the destination address.
- **SC-005**: A flood of 50 requests from one network address in one hour produces at most 5
  emails.
- **SC-006**: When sending fails, the user loses none of the typed text or the attachment
  (verified for no network, timeout, and refusal).
- **SC-007**: Running the report service costs nothing at the expected volume (up to 100
  reports a day).

## Assumptions

- The maintainer creates a free Cloudflare account and a free Resend account and deploys the
  service once; until then, the dialog uses the email fallback.
- The report service address is set at build time for releases. It is public by nature (the
  app calls it), so the rate limit and validation, not secrecy, protect it.
- Resend's free plan sends from its shared test sender or a domain the maintainer verifies;
  either is acceptable as the sender of reports.
- There is no reporter contact field: friends know the maintainer and reply happens outside
  the app. The name field identifies the reporter.
- One image per report is enough; more images can go in a follow-up message.
- The Portuguese message is a deliberate exception to the English-only UI rule, chosen by the
  maintainer; the plan records it as a constitution deviation.
- The log excerpt is the tail of today's Sonata log (or the latest one when today's is empty);
  agent terminal output is not part of Sonata's log.
- Reports are sent over HTTPS to the service; no network listener is added to the app.
