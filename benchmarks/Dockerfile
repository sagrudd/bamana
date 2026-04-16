FROM mambaorg/micromamba:1.5.10

ARG NXF_VER=24.10.5

USER root

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      bash \
      ca-certificates \
      coreutils \
      curl \
      gawk \
      git \
      openjdk-17-jre-headless \
      pandoc \
      procps \
      texlive-fonts-recommended \
      texlive-latex-base \
      texlive-latex-extra \
      texlive-latex-recommended \
      time \
      util-linux \
      lmodern && \
    rm -rf /var/lib/apt/lists/*

COPY benchmarks/environment.yml /tmp/environment.yml
RUN micromamba install -y -n base -f /tmp/environment.yml && \
    micromamba clean --all --yes

ENV NXF_VER=${NXF_VER}
RUN curl -fsSL https://get.nextflow.io | bash && \
    mv nextflow /usr/local/bin/nextflow && \
    chmod +x /usr/local/bin/nextflow

COPY benchmarks/bin/run_benchmark.sh /usr/local/bin/run_benchmark.sh
COPY benchmarks/bin/print_tool_versions.sh /usr/local/bin/print_tool_versions.sh
RUN chmod +x /usr/local/bin/run_benchmark.sh /usr/local/bin/print_tool_versions.sh

WORKDIR /workspace

CMD ["/bin/bash"]
