import type{AlivePrefs,CharacterId}from'./types';
const KEY='qa-lab-alive-v1';
export const defaults:AlivePrefs={enabled:true,selectionComplete:false,selectedCharacter:null,discoveredEasterEggs:[]};
export function loadPrefs():AlivePrefs{try{const value=JSON.parse(localStorage.getItem(KEY)||'{}');return{...defaults,...value,discoveredEasterEggs:Array.isArray(value.discoveredEasterEggs)?value.discoveredEasterEggs:[]}}catch{return{...defaults}}}
export function savePrefs(p:AlivePrefs){localStorage.setItem(KEY,JSON.stringify(p))}
export function selectCharacter(p:AlivePrefs,id:CharacterId):AlivePrefs{return{...p,selectionComplete:true,selectedCharacter:id}}
export function discover(p:AlivePrefs,id:string):AlivePrefs{return p.discoveredEasterEggs.includes(id)?p:{...p,discoveredEasterEggs:[...p.discoveredEasterEggs,id]}}
