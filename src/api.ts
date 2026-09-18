import { invoke } from '@tauri-apps/api/core';
import type {Catalog,DeviceCode,Tool} from './types';
export const api={
 session:()=>invoke<boolean>('has_session'),
 beginLogin:()=>invoke<DeviceCode>('begin_login'),
 finishLogin:(code:DeviceCode)=>invoke<void>('finish_login',{deviceCode:code.device_code,interval:code.interval,expiresIn:code.expires_in}),
 logout:()=>invoke<void>('logout'),
 catalog:()=>invoke<Catalog>('load_catalog'),
 install:(tool:Tool)=>invoke<void>('install_tool',{tool}),
 open:(tool:Tool)=>invoke<void>('open_tool',{tool}),
 openUrl:(url:string)=>invoke<void>('open_url',{url}),
};
