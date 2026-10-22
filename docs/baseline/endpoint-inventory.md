# Source endpoint inventory

Draft extracted from controller annotations. Not a Spring mapping export. Service calls are direct receivers only; entity trace and filter authorization need review. No method annotation does not imply public access. Framework routes and WebSocket are separate.

| Method | Route | Handler | UC / exclusion | Direct services | Method authorization |
|---|---|---|---|---|---|
| POST | `/ws/rest/client/getlink` | ClientAPI.getLinkV1 | UC-001 | sessionService | See WebRequestFilter |
| POST | `/ws/rest/client/link` | ClientAPI.getLink | UC-001 | sessionService | See WebRequestFilter |
| POST | `/ws/rest/v1/ai/inference` | InferenceAPI.runInference | UC-010 | aiAbnormalDetectionService | See WebRequestFilter |
| GET | `/ws/rest/v1/ai/model` | ModelController.getAll | UC-012 | modelService | See WebRequestFilter |
| GET | `/ws/rest/v1/ai/model/{id}` | ModelController.getById | UC-012 | modelService | See WebRequestFilter |
| POST | `/ws/rest/v1/ai/model` | ModelController.create | UC-012 | modelService | See WebRequestFilter |
| PUT | `/ws/rest/v1/ai/model` | ModelController.update | UC-012 | modelService | See WebRequestFilter |
| DELETE | `/ws/rest/v1/ai/model/{id}` | ModelController.delete | UC-012 | modelService | See WebRequestFilter |
| POST | `/ws/rest/v1/ai/request` | SegmentationAPI.requestSegmentation | UC-011 | aiSegmentationService | See WebRequestFilter |
| GET | `/ws/rest/v1/ai/status` | SegmentationAPI.getStatus | UC-011 | aiSegmentationService | See WebRequestFilter |
| POST | `/ws/rest/v1/ai/result` | SegmentationAPI.finalizeSegmentation | UC-011 | aiSegmentationService | See WebRequestFilter |
| GET | `/ws/rest/v1/ai/statistics` | SegmentationAPI.getStatistics | UC-011 | aiSegmentationService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/download/studies/{studyUID}` | DownloadAPI.downloadStudy | UC-005 | downloadService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/download/studies/{studyUID}/series/{seriesUID}` | DownloadAPI.downloadSeries | UC-005 | downloadService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/download/studies/{studyUID}/series/{seriesUID}/instances/{sopUID}` | DownloadAPI.downloadInstance | UC-005 | downloadService | See WebRequestFilter |
| GET | `/ws/rest/v1/license/ccu/events` | LicenseCcuAPI.events | UC-014 | ccuSseService | See WebRequestFilter |
| GET | `/ws/rest/v1/license/ccu/snapshot` | LicenseCcuAPI.snapshot | UC-014 | ccuSseService | See WebRequestFilter |
| POST | `/ws/rest/v1/license/ccu/heartbeat` | LicenseCcuAPI.heartbeat | UC-014 | ccuHeartbeatService | See WebRequestFilter |
| GET | `/ws/rest/v1/license/ccu/heartbeat/snapshot` | LicenseCcuAPI.heartbeatSnapshot | UC-014 | ccuHeartbeatService | See WebRequestFilter |
| GET | `/ws/rest/v1/license/status` | LicenseStatusAPI.getStatus | UC-014 | ccuSseService, licenseGuard | See WebRequestFilter |
| POST | `/ws/rest/v1/session/{sessionID}/print` | PrinterAPI.printRequest | UC-013 | printService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/printer` | PrinterAPI.getPrinters | UC-013 | printerCRUDService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/printer/{id}` | PrinterAPI.getPrinterById | UC-013 | printerCRUDService | See WebRequestFilter |
| POST | `/ws/rest/v1/session/{sessionID}/printer` | PrinterAPI.createPrinter | UC-013 | printerCRUDService | See WebRequestFilter |
| PUT | `/ws/rest/v1/session/{sessionID}/printer` | PrinterAPI.updatePrinter | UC-013 | printerCRUDService | See WebRequestFilter |
| DELETE | `/ws/rest/v1/session/{sessionID}/printer/{id}` | PrinterAPI.deletePrinter | UC-013 | printerCRUDService | See WebRequestFilter |
| POST | `/api/rt` | SealRuntimeAPI.handshake | excluded:seal | cryptoService, licenseGuard | See WebRequestFilter |
| GET | `/api/rc/{*chunkPath}` | SealRuntimeAPI.runtimeChunk | excluded:seal | SealedChunkService, licenseGuard, sealedChunkService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}` | SessionAPI.getByID | UC-002 | sessionService | See WebRequestFilter |
| DELETE | `/ws/rest/v1/session/{sessionID}` | SessionAPI.deleteByID | UC-002 | sessionService | See WebRequestFilter |
| POST | `/ws/rest/v1/session/{sessionID}/share` | SessionAPI.shareLink | UC-002 | sessionService | See WebRequestFilter |
| PUT | `/ws/rest/v1/session` | SessionAPI.updateSession | UC-002 | sessionService | See WebRequestFilter |
| POST | `/ws/rest/v1/session3d/websocketlink` | Session3DAPI.getWebSocketLink | UC-007 | session3DService | See WebRequestFilter |
| GET | `/ws/rest/v1/session3d/sessions` | Session3DAPI.getSessions | UC-007 | session3DService | See WebRequestFilter |
| GET | `/ws/rest/v1/session3d/sessions/{sessionID}` | Session3DAPI.getSession | UC-007 | session3DService | See WebRequestFilter |
| DELETE | `/ws/rest/v1/session3d/sessions/{sessionID}` | Session3DAPI.deleteSession | UC-007 | session3DService | See WebRequestFilter |
| GET | `/ws/rest/v1/session3d/resources` | Session3DAPI.getResource | UC-007 | session3DService | See WebRequestFilter |
| DELETE | `/ws/rest/v1/session3d/resources` | Session3DAPI.freeResource | UC-007 | session3DService | See WebRequestFilter |
| POST | `/ws/rest/v1/session/{sessionID}/studies` | StowRSAPI.store | UC-006 | stowRSService | See WebRequestFilter |
| POST | `/ws/rest/v1/study/search` | StudyAPI.searchStudy | UC-003 | studyService | See WebRequestFilter |
| GET | `/ws/rest/v1/admin/client` | AdminAPI.getClients | UC-009 | clientService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/client/{id}` | AdminAPI.getClientById | UC-009 | clientService | "hasRole('ADMIN')" |
| POST | `/ws/rest/v1/admin/client` | AdminAPI.createClient | UC-009 | clientService | "hasRole('ADMIN')" |
| PUT | `/ws/rest/v1/admin/client` | AdminAPI.updateClient | UC-009 | clientService | "hasRole('ADMIN')" |
| DELETE | `/ws/rest/v1/admin/client/{id}` | AdminAPI.deleteClient | UC-009 | clientService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/client/{clientId}/permission` | AdminAPI.getClientPermissions | UC-009 | clientPermissionService | "hasRole('ADMIN')" |
| POST | `/ws/rest/v1/admin/client/{clientId}/permission` | AdminAPI.addClientPermission | UC-009 | clientPermissionService | "hasRole('ADMIN')" |
| DELETE | `/ws/rest/v1/admin/client/{clientId}/permission/{permissionId}` | AdminAPI.removeClientPermission | UC-009 | clientPermissionService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/config` | AdminAPI.getConfigs | UC-008 | configService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/config/{id}` | AdminAPI.getConfigById | UC-008 | configService | "hasRole('ADMIN')" |
| POST | `/ws/rest/v1/admin/config` | AdminAPI.createConfig | UC-008 | configService | "hasRole('ADMIN')" |
| PUT | `/ws/rest/v1/admin/config` | AdminAPI.updateConfig | UC-008 | configService | "hasRole('ADMIN')" |
| DELETE | `/ws/rest/v1/admin/config/{id}` | AdminAPI.deleteConfig | UC-008 | configService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/permission` | AdminAPI.getPermissions | UC-009 | permissionCrudService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/admin/permission/{id}` | AdminAPI.getPermissionById | UC-009 | permissionCrudService | "hasRole('ADMIN')" |
| POST | `/ws/rest/v1/admin/permission` | AdminAPI.createPermission | UC-009 | permissionCrudService | "hasRole('ADMIN')" |
| PUT | `/ws/rest/v1/admin/permission` | AdminAPI.updatePermission | UC-009 | permissionCrudService | "hasRole('ADMIN')" |
| DELETE | `/ws/rest/v1/admin/permission/{id}` | AdminAPI.deletePermission | UC-009 | permissionCrudService | "hasRole('ADMIN')" |
| GET | `/ws/rest/v1/session/{sessionId}/config` | ConfigPublicAPI.getConfigs | UC-008 | configService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionId}/config/{id}` | ConfigPublicAPI.getConfigById | UC-008 | configService | See WebRequestFilter |
| POST | `/ws/rest/v1/session/{sessionId}/config` | ConfigPublicAPI.createConfig | UC-008 | configService | "@sessionPermissionEvaluator.hasPermission('" + PermissionKey.MODIFY_OVERLAY_CONFIG + "')" |
| PUT | `/ws/rest/v1/session/{sessionId}/config` | ConfigPublicAPI.updateConfig | UC-008 | configService | "@sessionPermissionEvaluator.hasPermission('" + PermissionKey.MODIFY_OVERLAY_CONFIG + "')" |
| DELETE | `/ws/rest/v1/session/{sessionId}/config/{id}` | ConfigPublicAPI.deleteConfig | UC-008 | configService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}` | WadoRsAPI.getStudy | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/thumbnail` | WadoRsAPI.getStudyThumbnail | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/metadata` | WadoRsAPI.getStudyMetadata | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series` | WadoRsAPI.getSeries | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}` | WadoRsAPI.getSeries | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/thumbnail` | WadoRsAPI.getSeriesThumbnail | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/metadata` | WadoRsAPI.getSeriesMetadata | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/instances` | WadoRsAPI.getInstances | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/instances/{objectUID}` | WadoRsAPI.getInstances | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/instances/{objectUID}/frames/{frames}/thumbnail` | WadoRsAPI.getInstanceFrameThumbnail | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-rs/studies/{studyUID}/series/{seriesUID}/instances/{objectUID}/{bulk}/{bulkData}` | WadoRsAPI.getBulkData | UC-004 | wadoRsService | See WebRequestFilter |
| GET | `/ws/rest/v1/session/{sessionID}/wado-uri` | WadoRsAPI.getInstanceByWadoURI | UC-004 | wadoRsService | See WebRequestFilter |

## Additional surfaces requiring runtime reconciliation

- `/ws/*`: WebSocketConfig → WebSocketHandler; UC-007.
- `/v3/api-docs/**`, Swagger UI, framework error and actuator routes: infrastructure; effective availability requires runtime export.
- QIDO service/Feign mappings are upstream calls, not standalone incoming controller routes.
- Seal routes remain listed with `excluded:seal` for scope reconciliation.
