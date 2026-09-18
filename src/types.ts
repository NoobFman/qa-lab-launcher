export type ToolStatus='not_installed'|'updated'|'update_available';
export interface Tool { owner:string; repo:string; name:string; description:string; icon_url?:string; remote_version:string; local_version?:string; status:ToolStatus; asset_name:string; }
export interface Catalog { tools:Tool[]; unconfigured:number; warnings:string[]; }
export interface DeviceCode { device_code:string; user_code:string; verification_uri:string; expires_in:number; interval:number; }
