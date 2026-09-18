import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import {LabAlivePreview} from './features/lab-alive/LabAlivePreview';
import './styles.css';
import './update.css';
const preview=import.meta.env.DEV&&new URLSearchParams(location.search).has('alive-preview');
ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode>{preview?<LabAlivePreview/>:<App/>}</React.StrictMode>);
