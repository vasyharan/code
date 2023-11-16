FROM gitpod/workspace-base

USER root

# Install Nix
COPY ./.gitpod/nix-install.sh /tmp/

RUN addgroup --system nixbld \
  && adduser gitpod nixbld \
  && for i in $(seq 1 30); do useradd -ms /bin/bash nixbld$i &&  adduser nixbld$i nixbld; done \
  && mkdir -m 0755 /nix && chown gitpod /nix \
  && mkdir -p /etc/nix && echo 'sandbox = false' > /etc/nix/nix.conf

# Install Nix
USER gitpod
ENV USER gitpod
WORKDIR /home/gitpod

RUN set -eux; \
  touch .bash_profile \
  && cat /tmp/nix-install.sh | sh \
  && echo '. /home/gitpod/.nix-profile/etc/profile.d/nix.sh' >> $HOME/.bashrc

RUN set -eux; \
  mkdir -p $HOME/.config/nixpkgs \
  && mkdir -p $HOME/.config/nix \
  && echo '{ allowUnfree = true; }' >> $HOME/.config/nixpkgs/config.nix \
  && echo 'sandbox = false' >> $HOME/.config/nix/nix.conf \
  && echo 'experimental-features = nix-command flakes' >> $HOME/.config/nix/nix.conf

# Install cachix/devenv/direnv/git
RUN set -eux; \
  . $HOME/.nix-profile/etc/profile.d/nix.sh \
  && nix-env -iA cachix -f https://cachix.org/api/v1/install \
  && cachix use devenv \
  && nix-env -if https://install.devenv.sh/latest \
  && nix-env -i direnv git git-lfs \
  && mkdir -p $HOME/.config/direnv \
  && printf '%s\n' '[whitelist]' 'prefix = [ "/workspace"] ' >> $HOME/.config/direnv/config.toml \
  && printf '%s\n' 'source <(direnv hook bash)' >> $HOME/.bashrc.d/999-direnv

COPY ./devenv.* ./
RUN set -eux; \
  . $HOME/.nix-profile/etc/profile.d/nix.sh \
  && devenv ci

CMD /bin/bash -l