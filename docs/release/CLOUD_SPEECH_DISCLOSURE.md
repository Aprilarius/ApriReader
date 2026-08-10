# Cloud speech disclosure

This record keeps marketplace descriptions consistent with the application.
ApriReader does not bundle voices, sample books, narration, or generated store
artwork.

## Optional speech services

Local read-aloud uses voices already installed in Windows. A user may instead
choose ElevenLabs, Google Cloud Text-to-Speech, or Azure AI Speech and provide
their own account key. These services turn supplied text into speech audio; they
do not write or alter the source book.

- Cloud speech is off by default.
- The user selects the provider and accepts a provider-specific notice before
  any text leaves the device.
- Only bounded fragments of the chosen local book are submitted.
- Requests use fixed provider hosts and bounded input and output.
- Credentials remain protected by Windows Credential Manager.
- Provider terms, quota, billing, and permission to process the selected text
  remain the user's responsibility.

ApriReader has no general-purpose image, text, or media creation feature and
does not send library content to a network service in the background.
