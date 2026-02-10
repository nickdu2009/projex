import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { Layout } from './pages/Layout';
import { PartnerDetail } from './pages/PartnerDetail';
import { PartnerForm } from './pages/PartnerForm';
import { PartnersList } from './pages/PartnersList';
import { PeopleList } from './pages/PeopleList';
import { PersonDetail } from './pages/PersonDetail';
import { PersonForm } from './pages/PersonForm';
import { ProjectDetail } from './pages/ProjectDetail';
import { ProjectForm } from './pages/ProjectForm';
import { ProjectsList } from './pages/ProjectsList';
import { Settings } from './pages/Settings';

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Navigate to="/projects" replace />} />
          <Route path="projects" element={<ProjectsList />} />
          <Route path="projects/new" element={<ProjectForm />} />
          <Route path="projects/:id" element={<ProjectDetail />} />
          <Route path="projects/:id/edit" element={<ProjectForm />} />
          <Route path="people" element={<PeopleList />} />
          <Route path="people/new" element={<PersonForm />} />
          <Route path="people/:id" element={<PersonDetail />} />
          <Route path="people/:id/edit" element={<PersonForm />} />
          <Route path="partners" element={<PartnersList />} />
          <Route path="partners/new" element={<PartnerForm />} />
          <Route path="partners/:id" element={<PartnerDetail />} />
          <Route path="partners/:id/edit" element={<PartnerForm />} />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
