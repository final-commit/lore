import { redirect } from 'next/navigation'

export default function Home() {
  // Redirect to docs root
  redirect('/docs')
}
