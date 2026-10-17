# Read-only kernel module inventory

ET_REL symbol values below are section-relative, not runtime virtual addresses. Search covered full .symtab names and printable ASCII strings for cover, picture, metadata, GINF, CNTINF, thumbnail, jacket, GPIF, GPFB. No literal cover/database parser names or strings were found in these three modules. Broad `pic` regex matches in mg.ko (e.g. mgApiCalc*) are substring false positives. This is a bounded negative search, not proof that no unnamed parsing exists.

## mg.ko

- SHA-256: `72e5a18e3f291ab54f907d626a6b81427f10210d6d5f6274edb464274f153d57`
- Size/type/symbols: 68920 bytes, ET_REL, EM_ARM, 571 .symtab entries

Function-name scope: `MgipIoctl`, `MgipMmap`, `MgipOpen`, `MgipClose`, `MgipInit`, `MgipCleanup`, `MODULE_PARM_`, `initializeHandle`, `mgNewKtObject`, `mgipNandReadLeafId16`, `ddmgAesCmpVcv`, `mgApiGetEkbVersion`, `ddmgGenKleaf`, `ddmgEncKicv2KrootKicv`, `ddmgCalcMac`, `ddmgCopyKroot`, `mgipNandGetEmprEKBSize`, `ddmgKbEncCbc`, `ddmgAesEncCbc`, `ddmgAesKeyClear`, `ddmgAesGenKt`, `mgipNandWriteProductGroupScramble`, `mgipNandWriteDNK`, `msd3DesCalcMacInit`, `mgApiReserveKbObject`, `ddmgAesCalcMac`, `ddmgHardReset`, `mgGetKrootObject`, `mgipNandReadDestination`, `mgipNandReadMediaId`, `mgGetKcObject`, `mgDksGetLeafIdU`, `ddmgAesDecKrootKc2Kc`, `ddmgGenRandomNumber`, `msd3DesCalcMacSetData`, `mgApiAesDecData`, `ddmgDecKrootKicv2Kicv`, `mgApiAesGenKicv`, `ddmgDecCbc`, `mgInitialize`, `mgApiFreeKrootObject`, `mgipNandGetAadEKBSize`, `ddmgLoadRndSeed`, `cleanup_module`, `mgClrKbObject`, `ddmgEncEcb`, `mgApiCalcIcvStep1`, `mgApiDecData`, `mgipNandReadIcvStorage`, `mgipNandReadEmprEKB`, `ddmgSaveRndSeed`, `mgApiSetKbKp`, `mgipNandWriteEmprEKB`, `mgDksGetLeafId`, `mgDksGetMgId`, `mgApiReserveLocalEkbInf`, `init_module`, `mgipNandWriteAadEKB`, `mgNewKbObject`, `mgApiReserveKtObject`, `mgNewKcObject`, `mgApiAesEncData`, `mgApiCopyKroot`, `ddmgFastKbDecCbc`, `mgApiSetKbSeedC`, `mgGetLocalEkbInf`, `mgGetKtObject`, `ddmgAesGenKicv`, `mgApiDecEcb`, `mgApiCalcIcvStep2`, `mgApiFreeKbObject`, `ddmgKbEncCbc2`, `mgipNandWriteIcvStorage`, `mgInitializeDeepSleep`, `mgApiDecFw`, `mgipNandReadAadEKB`, `ddmg3DesCalcMac`, `ddmgAesDecCbcSimple`, `mgipNandInitICVStorage`, `mgApiDecKrootByKroot`, `ddmgDecEcb`, `mgApiCalcMac`, `mgApi3DesCalcMac`, `ddmgKfree`, `ddmgCmpEkbVersion`, `mgApiEncEcb`, `mgipNandReadProductId`, `mgClrKcObject`, `mgipNandReadLeafIdL8`, `mgApiReserveKcObject`, `mgShrinkEkb`, `mgApiAesCalcMac`, `ddmgGenKc`, `mgApiFreeKtObject`, `ddmgGenKC2`, `mgApiAesDecKey128`, `mgipNandWriteTestDNK`, `mgDksLoadDKS`, `mgipNandWriteSerialNum`, `mgClrLocalEkbInf`, `mgApiSetKbSeedF`, `ddmgGetParameters`, `mgApi3DesDecKey64`, `mgGetKbObject`, `mgApiMakeKrootFromLocalEkb`, `mgApiEncData`, `ddmgDecKnode`, `ddmgGenKst1l`, `msd3DesCalcMacEnd`, `ddmgInitializeParameters`, `mgipNandReadSerialNum`, `mgApiMakeKroot`, `ddmgInitialize`, `mgApiFreeLocalEkbInf`, `mgApiGetRnd`, `mgApiAesGenKt`, `mgipNandReadProductGroupScramble`, `mgApiAesGenKmac`, `ddmgKbDecCbc`, `ddmgAesGenKmac`, `ddmgDecKnodeByKleaf`, `mgClrKtObject`, `mgApiFreeKcObject`, `ddmgCalcMacPlainKey`, `mgApiDecFringe`, `mgNewLocalEkbInf`, `mgApi3DesDecKey128`, `ddmgGenKb`, `ddmgDecKrootKc2Kc`, `mgipNandReadDNK`, `mgApiReserveKrootObject`, `ddmgGenKr`, `mgClrKrootObject`, `mgipNandReadModelId`, `mgipNandReadDNK_MediaID`, `mgApiEncKrootByKroot`, `ddmgAesDecCbc`, `ddmgdSelectHiddenKey`, `mgDksGetLeafIdL`, `mgApiCalcIcvStep3`, `mgipNandWriteProductId`, `mgDksSetMgId`, `mgApiGetKeyStatus`, `ddmgDecKleafKnode2Knode`, `mgNewKrootObject`, `ddmgAesEncCbcSimple`, `mgApiShrinkEkb`, `mgDksGetRndSeed`, `mgApiCalcMacPlainKey`, `GetBigEndian32`

## mgscsi.ko

- SHA-256: `27e9ae59a30d1c3999534838010b1288840c787b798fe46bc58657b7eb30cac6`
- Size/type/symbols: 66552 bytes, ET_REL, EM_ARM, 503 .symtab entries

| Symbol | Section offset | Size |
|---|---:|---:|
| `mgscsi_ioctl_get_P_LST_C` | `.text+0x7c0` | 84 |
| `mgscsi_ioctl_set_BFL_REQ` | `.text+0xa04` | 128 |
| `mgscsi_ioctl_get_V_SCHCI` | `.text+0xca8` | 84 |
| `mgscsi_ioctl_get_DevCertTemplate` | `.text+0x1714` | 424 |
| `mgscsi_ioctl_set_EMPR_8100` | `.text+0x11e8` | 124 |
| `mgscsi_ioctl_set_RSV_ARA` | `.text+0x90c` | 40 |
| `mgscsi_ioctl_get_BFL_REQ` | `.text+0xa84` | 92 |
| `mgscsi_ioctl_set_P_LST_D` | `.text+0x890` | 40 |
| `mgscsi_ioctl_set_EMPR_0121` | `.text+0xf78` | 124 |
| `mgscsi_ioctl_set_DEVINFO` | `.text+0xae0` | 124 |
| `mgscsi_ioctl_get_EMPR_8101` | `.text+0x1334` | 84 |
| `mgscsi_ioctl_get_EMPR_0100` | `.text+0xe54` | 84 |
| `mgscsi_ioctl_set_EMPR_0120` | `.text+0xea8` | 124 |
| `mgscsi_ioctl_get_DEVINFO` | `.text+0xb5c` | 84 |
| `mgscsi_ioctl_get_USERCLK` | `.text+0xbd8` | 84 |
| `mgscsi_ioctl_get_R_GTCLG` | `.text+0x9b0` | 84 |
| `mgscsi_Process_ModeSenseCommand` | `.text+0x6c4` | 120 |
| `mgscsi_ioctl_set_EMPR_8101` | `.text+0x12b8` | 124 |
| `mgscsi_ioctl_set_EMPR_0160` | `.text+0x1118` | 124 |
| `mgscsi_ioctl_get_P_LST_D` | `.text+0x8b8` | 84 |
| `mgscsi_ioctl_get_EMPR_0140` | `.text+0x10c4` | 84 |
| `mgscsi_ioctl_set_R_GTCLG` | `.text+0x988` | 40 |
| `mgscsi_ioctl` | `.text+0x1a0` | 1056 |
| `mgscsi_ioctl_set_conn_status` | `.text+0x1458` | 48 |
| `mgscsi_Process_VendorSpecificCommand` | `.text+0x5dc` | 124 |
| `mgscsi_ioctl_set_EMPR_0140` | `.text+0x1048` | 124 |
| `mgscsi_ioctl_get_EMPR_8100` | `.text+0x1264` | 84 |
| `mgscsi_ioctl_set_EMPR_8200` | `.text+0x1388` | 124 |
| `mgscsi_ioctl_get_EMPR_0120` | `.text+0xf24` | 84 |
| `mgscsi_ioctl_get_EMPR_0121` | `.text+0xff4` | 84 |
| `mgscsi_ioctl_set_EMPR_0100` | `.text+0xdd8` | 124 |
| `mgscsi_Process_InquiryCommand` | `.text+0x658` | 108 |
| `mgscsi_ioctl_set_V_AFRQC` | `.text+0xcfc` | 128 |
| `mgscsi_ioctl_get_P_LST_S` | `.text+0x83c` | 84 |
| `mgscsi_ioctl_set_V_SCHCI` | `.text+0xc2c` | 124 |
| `mgscsi_ioctl_get_V_AFRQC` | `.text+0xd7c` | 92 |
| `mgscsi_ioctl_get_EMPR_0160` | `.text+0x1194` | 84 |
| `mgscsi_ioctl_set_P_LST_C` | `.text+0x798` | 40 |
| `mgscsi_ioctl_get_PrivateKey` | `.text+0x15e4` | 304 |
| `mgscsi_ioctl_set_P_LST_S` | `.text+0x814` | 40 |
| `mgscsi_ioctl_set_USERCLK` | `.text+0xbb0` | 40 |
| `mgscsi_ioctl_get_RSV_ARA` | `.text+0x934` | 84 |
| `mgscsi_ioctl_get_EMPR_8200` | `.text+0x1404` | 84 |

Function-name scope: `vscGetEmprPropPageSize`, `vscGetNumOfPropPage`, `vscDnkScrambleData`, `vscDnkGet_eDKS_Size`, `mgscsiAuthCalcResponse`, `vscEmprDpccProcess8101`, `_GetProductInfo`, `mgscsiMgCtrl3DesCalcMac`, `SetBigEndian32`, `vscEmprDpccProcess0121`, `vscA4CheckCmd`, `vscProcessCmd`, `mgscsi_ioctl_get_P_LST_C`, `mgscsi_ioctl_set_BFL_REQ`, `mgscsiAuthGetSecureSettingFlagStatus`, `mgscsi_ioctl_get_V_SCHCI`, `vscDpccProcessV_AFRQC`, `vscDnkCheckCmd`, `mgscsi_ioctl_get_DevCertTemplate`, `mgscsiAuthSetSecureSettingFlag`, `mgscsi_ioctl_set_EMPR_8100`, `cleanup_module`, `mgscsiAuthGetInitialEkbVersion`, `mgscsi_ioctl_set_RSV_ARA`, `vscDpccCheckCmd`, `mgscsiAuthGetDeviceLocalEkbDepth`, `mgscsi_ioctl_get_BFL_REQ`, `vscDnkProcessCmd`, `mgscsi_ioctl_set_P_LST_D`, `mgscsi_ioctl_set_EMPR_0121`, `mgscsiAuthStartSession`, `vscA4ProcessCmd`, `mgscsi_ioctl_set_DEVINFO`, `mgscsiAuthSetAuthStatus`, `b2a`, `mgscsi_ioctl_get_EMPR_8101`, `vscEmprDpccProcessCmd`, `mgscsiAuthUnsetSecureSettingFlag`, `init_module`, `vscDnkGetReplySize`, `mgscsiAuthWriteIcvStorage`, `vscDpccProcessBFL_REQ`, `mgscsi_ioctl_get_EMPR_0100`, `mgscsi_ioctl_set_EMPR_0120`, `vscUserTimeProcessCmd`, `vscDpccProcessV_SCHCI`, `vscEmprDpccCheckCmd`, `mgscsi_ioctl_get_DEVINFO`, `mgscsi_ioctl_get_USERCLK`, `mgscsi_ioctl_get_R_GTCLG`, `vscEmprDpccProcess0120`, `vscDpccProcessCmd`, `mgscsiAuthCalcMac`, `mgscsi_Process_ModeSenseCommand`, `vscDnkProcessReadRegSerialNum`, `mgscsiMgCtrlCalcMacPlainKey`, `vscDpccGetReplySize`, `mgscsiAuthGetSessionKey`, `mgscsi_ioctl_set_EMPR_8101`, `vscEmprDpccProcess8200`, `SetBigEndian16`, `mgscsiAuthGetDeviceLocalEkb`, `mgscsi_ioctl_set_EMPR_0160`, `mgscsi_ioctl_get_P_LST_D`, `vscEmprDpccProcess0100`, `mgscsiAuthInit`, `register_mgscsi_func`, `vscA3ProcessCmd`, `vscA4GetReplySize`, `mgscsi_ioctl_get_EMPR_0140`, `mgscsi_ioctl_set_R_GTCLG`, `mgscsi_ioctl`, `mgscsi_ioctl_set_conn_status`, `vscDpccProcessP_LST_D`, `vscA3CheckCmd`, `mgscsi_Process_VendorSpecificCommand`, `vscEmprDpccProcess8100`, `mgscsi_ioctl_set_EMPR_0140`, `vscDpccProcessP_LST_S`, `mgscsi_ioctl_get_EMPR_8100`, `mgscsi_ioctl_set_EMPR_8200`, `mgscsi_ioctl_get_EMPR_0120`, `vscDpccProcessUSERCLK`, `vscInit`, `mgscsiMgCtrlDecEcb`, `mgscsi_ioctl_get_EMPR_0121`, `vscEmprDpccProcess0000`, `vscCheckCmd`, `mgscsi_ioctl_set_EMPR_0100`, `mgscsiMgCtrlInit`, `vscDpccProcessP_LST_C`, `mgscsi_init`, `mgscsi_Process_InquiryCommand`, `mgscsi_exit`, `vscEmprDpccProcess0140`, `mgscsiAuthCreateSessionKey`, `vscDpccProcessRSV_ARA`, `mgscsiAuthGetAuthStatus`, `vscUserTimeCheckCmd`, `mgscsi_ioctl_set_V_AFRQC`, `mgscsi_ioctl_get_P_LST_S`, `mgscsiMgCtrlMakeKrootFromLocalEkb`, `vscEmprDpccProcess0160`, `mgscsi_ioctl_set_V_SCHCI`, `GetBigEndian16`, `mgscsi_ioctl_get_V_AFRQC`, `mgscsi_ioctl_get_EMPR_0160`, `mgscsi_ioctl_set_P_LST_C`, `vscEmprDpccGetReplySize`, `vscDpccProcessR_GTCLG`, `mgscsiAuthGetDeviceResponse`, `mgscsiMgCtrlGetRandomNumber`, `mgscsi_ioctl_get_PrivateKey`, `mgscsi_ioctl_set_P_LST_S`, `mgscsi_ioctl_set_USERCLK`, `mgscsi_ioctl_get_RSV_ARA`, `mgscsiAuthSetHostLocalEkb`, `mgscsi_get_drm`, `mgscsi_ioctl_get_EMPR_8200`, `vscDpccProcessDEVINFO`, `mgscsiAuthGetDeviceLocalEkbKeyNum`, `vscUserTimeGetReplySize`, `unregister_mgscsi_func`, `GetBigEndian32`, `mgscsiMgCtrlEncEcb`

## mgscc.ko

- SHA-256: `6d21a130941d56ee5b10b1d31781b7a13ac1db09f0bafd6655476b01b65f0c8d`
- Size/type/symbols: 18412 bytes, ET_REL, EM_ARM, 183 .symtab entries

Function-name scope: `mgscc_init`, `mgscc_exit`, `mgscc_ioctl`, `mgscc_open`, `mgscc_release`, `sec2time`, `setLinuxTime`, `sysTmstInitTimeDifAll`, `sysTmstIsRtcStopped`, `sysTmstInitTimeDif`, `sysTmstIsTrustEnable`, `sysTmstTrustEnable`, `check_time`, `time2sec`, `update_timezone`, `sysTmstSetEnable`, `get_rtc_time_abs`, `set_rtc_time_abs`, `sysTmstInitRtc`, `sysTmstSetClockTimeStamp`, `sysTmstGetClockTimeStamp`, `trasnfer_timezone`, `call_back_func`, `sysTmstIsAadTrustEnable`, `cleanup_module`, `sysTmstSetSecureSettingFlag`, `sysTmstGetSecureSettingStatus`, `init_module`, `sysTmstUsrTrustEnable`, `sysTmstConvBCD2INT`, `sysTmstCheckRoundTrip`, `sysTmstGetAadClockTimeStamp`, `Process_SccCommand`, `sysTmstWmtTrustEnable`, `sysTmstAadTrustEnable`, `sysTmstSetUsrClockTimeStamp`, `sysTmstIsUsrTrustEnable`, `sysTmstIsWmtTrustEnable`, `sysTmstSetAadClockTimeStamp`, `sysTmstConvINT2BCD`, `sysTmstGetUsrClockTimeStamp`

## Interpretation

mg.ko function names concern cryptographic/authentication services. mgscsi.ko retains ioctl buffer setters/getters, vendor-specific SCSI command processing and authentication. mgscc.ko retains secure-clock/SCC time, trust-setting, RTC and timezone functions. No module presents a named artwork/03GINF/04CNTINF parser target. For the main executable AdtMgScsi_setup template, use mgscsi_ioctl and ioctl buffer copy routines only to identify metadata configuration semantics; these symbols alone do not establish firmware artwork read behavior.
