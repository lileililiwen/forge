# {{project_name}} container runtime

This runtime serves the already-built `{{output_dir}}` directory with nginx.
The generated health check probes `{{health_path}}`.

Run the project's build before `docker compose build`; this template does not
silently invent a frontend toolchain or build output.
