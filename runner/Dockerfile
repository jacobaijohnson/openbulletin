# OpenBulletin self-hosted runner image.
# Deliberately separate from the EV project's ev-runner: own image, own
# container, own registration, own caches. Based on the same public
# myoung34/github-runner base the EV runner uses, minus its SDL pieces.
# Base pinned to a named version (audit Batch 3): a version tag, not a
# digest, so the recreate flow in README.md keeps working unchanged;
# bump the version on purpose, never by drift.
FROM myoung34/github-runner:2.338.0

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Build deps: Rust toolchain needs + egui/winit Linux GUI libs (same set
# the GitHub-hosted CI installs, so local and hosted builds agree).
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential pkg-config git curl ca-certificates \
      libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
      libxkbcommon-dev libssl-dev libwayland-dev \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p "$RUSTUP_HOME" "$CARGO_HOME" \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
       | sh -s -- -y --default-toolchain 1.99.0 --profile minimal \
         --component rustfmt --component clippy --no-modify-path \
    && chmod -R a+rX "$RUSTUP_HOME" "$CARGO_HOME" \
    && rustc --version && cargo --version
