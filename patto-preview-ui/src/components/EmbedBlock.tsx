import { isGooglePhotosLink, pdfSrc, SLIDE_SHARE, SPEAKER_DECK } from '../embeds';
import OEmbedBlock from './OEmbedBlock';
import TwitterBlock from './TwitterBlock';
import PdfBlock from './PdfBlock';
import GooglePhotosBlock from './GooglePhotosBlock';
import IframeEmbed from './IframeEmbed';

interface EmbedBlockProps {
    link: string;
    title: string | null;
}

export default function EmbedBlock({ link, title }: EmbedBlockProps) {
    if (link.toLowerCase().endsWith('.pdf')) {
        return <PdfBlock src={pdfSrc(link)} title={title} />;
    }
    if (link.includes('speakerdeck.com')) {
        return <OEmbedBlock url={link} service={SPEAKER_DECK} />;
    }
    if (link.includes('slideshare.net')) {
        return <OEmbedBlock url={link} service={SLIDE_SHARE} />;
    }
    if (link.includes('twitter.com') || link.includes('x.com')) {
        return <TwitterBlock url={link} title={title} />;
    }
    if (isGooglePhotosLink(link)) {
        return <GooglePhotosBlock url={link} title={title} />;
    }
    return <IframeEmbed link={link} title={title} />;
}
