import { importExportApi } from "./apiModules/importExportApi";
import { recordTagsApi } from "./apiModules/recordTagsApi";
import { settingsApi } from "./apiModules/settingsApi";
import { tableApi } from "./apiModules/tableApi";
import { viewLayoutApi } from "./apiModules/viewLayoutApi";
import { viewNavigationApi } from "./apiModules/viewNavigationApi";

export const api = {
  ...settingsApi,
  ...tableApi,
  ...importExportApi,
  ...viewNavigationApi,
  ...viewLayoutApi,
  ...recordTagsApi
};
