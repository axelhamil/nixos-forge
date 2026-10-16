{ config, ... }:

let
  inherit (config.helpers) mkDefaults relativeToHome;
in
{
  apps.firefox.homeManager =
    { config, ... }:
    {
      programs.firefox = {
        enable = true;
        policies = mkDefaults {
          DisableTelemetry = true;
          DisableFirefoxStudies = true;
          DontCheckDefaultBrowser = true;
          EnableTrackingProtection = {
            Value = true;
            Cryptomining = true;
            Fingerprinting = true;
            EmailTracking = true;
          };
          FirefoxHome = {
            SponsoredTopSites = false;
            SponsoredStories = false;
          };
          FirefoxSuggest = {
            SponsoredSuggestions = false;
            ImproveSuggest = false;
          };
          UserMessaging = {
            ExtensionRecommendations = false;
            FeatureRecommendations = false;
            SkipOnboarding = true;
          };
        };
      };

      home.persistence."/persist".directories = [
        (relativeToHome config config.programs.firefox.configPath)
      ];
    };
}
