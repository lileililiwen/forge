FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
ARG APP_PROJECT={{app_project}}
WORKDIR /src
COPY . .
RUN dotnet restore "$APP_PROJECT"
RUN dotnet publish "$APP_PROJECT" -c Release --no-restore -o /out /p:UseAppHost=false

FROM mcr.microsoft.com/dotnet/aspnet:10.0 AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser
COPY --from=build /out .
USER appuser
ENV ASPNETCORE_URLS=http://+:8080
EXPOSE 8080
ENTRYPOINT ["dotnet", "{{project_name}}.dll"]
