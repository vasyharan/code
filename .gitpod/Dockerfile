FROM gitpod/workspace-base:2024-03-31-14-01-15

ARG NIX_VERSION="2.19.4"
ARG NIX_CONFIG="experimental-features = nix-command flakes"

ENV NIX_VERSION=${NIX_VERSION}

USER root

RUN addgroup --system nixbld \
  && adduser gitpod nixbld \
  && for i in $(seq 1 30); do useradd -ms /bin/bash nixbld$i && adduser nixbld$i nixbld; done \
  && mkdir -m 0755 /nix && chown gitpod /nix \
  && mkdir -p /etc/nix && echo 'sandbox = false' > /etc/nix/nix.conf

# Install Nix
USER gitpod
ENV USER gitpod
WORKDIR /home/gitpod

RUN curl https://nixos.org/releases/nix/nix-$NIX_VERSION/install | sh

RUN echo '. /home/gitpod/.nix-profile/etc/profile.d/nix.sh' >> /home/gitpod/.bashrc.d/200-nix
RUN mkdir -p /home/gitpod/.config/nixpkgs && echo '{ allowUnfree = true; }' >> /home/gitpod/.config/nixpkgs/config.nix
RUN mkdir -p /home/gitpod/.config/nix && echo $NIX_CONFIG >> /home/gitpod/.config/nix/nix.conf

# Install cachix
RUN . /home/gitpod/.nix-profile/etc/profile.d/nix.sh \
  && nix-env -iA cachix -f https://cachix.org/api/v1/install \
  && cachix use cachix

# Install nixpkgs
RUN VERSION=5753c7e1d88bad3dc6c12344974e0c5823aaf2c9 \
  && curl -Lo nixpkgs.zip "https://github.com/vasyharan/nixpkgs/archive/$VERSION.zip" \
  && unzip nixpkgs.zip \
  && mv "nixpkgs-$VERSION" nixpkgs \
  && cd nixpkgs \
  && . /home/gitpod/.nix-profile/etc/profile.d/nix.sh \
  && nix run -v github:nix-community/home-manager -- switch --flake '.#gitpod' \
  && echo ". /home/gitpod/.nix-profile/etc/profile.d/nix.sh" | sudo tee -a /home/gitpod/.config/zsh/.zshrc > /dev/null \
  && sudo chsh "$(id -un)" --shell "$(which zsh)" \
  && nvim --headless +TSUpdateSync +qa
COPY <<EOF /home/gitpod/.config/direnv/config.toml
[whitelist]
prefix = [ "/workspace" ]
EOF

COPY ./flake.* /tmp/flake/
RUN cd /tmp/flake/ \
  && . /home/gitpod/.nix-profile/etc/profile.d/nix.sh \
  && nix develop --accept-flake-config --command bash -c 'true' 
