import { useState} from 'react';
import { Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { MouseEvent } from 'react';

interface CreateSinkModalProps {
  onCreateSink: (name: string, description: string) => void;
}

export function CreateSinkModal({ onCreateSink }: CreateSinkModalProps) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  const handleCreate = (e: MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    if (name.trim()) {
      onCreateSink(name, description);
      setName('');
      setDescription('');
      setOpen(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button className='bg-purple-600 hover:bg-purple-700 dark:bg-purple-500 dark:hover:bg-purple-600 text-white'>
          <Plus className='h-4 w-4 mr-2' />
          Create Virtual Sink
        </Button>
      </DialogTrigger>
      <DialogContent className='sm:max-w-[425px] bg-background border-border'>
        <DialogHeader>
          <DialogTitle className='text-foreground'>
            Create New Virtual Sink
          </DialogTitle>
          <DialogDescription className='text-muted-foreground'>
            Create a new virtual audio sink for advanced audio routing.
          </DialogDescription>
        </DialogHeader>
        <div className='grid gap-4 py-4'>
          <div className='grid grid-cols-4 items-center gap-4'>
            <Label htmlFor='sink-name' className='text-right text-foreground'>
              Name
            </Label>
            <Input
              id='sink-name'
              value={name}
              onChange={(e) => setName(e.target.value)}
              className='col-span-3 bg-background border-border text-foreground'
              placeholder='Virtual Sink Name'
            />
          </div>
          <div className='grid grid-cols-4 items-center gap-4'>
            <Label
              htmlFor='sink-description'
              className='text-right text-foreground'
            >
              Description
            </Label>
            <Input
              id='sink-description'
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className='col-span-3 bg-background border-border text-foreground'
              placeholder='Virtual Sink Description'
            />
          </div>
          {/* <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="channels" className="text-right text-foreground">
              Channels
            </Label>
            <Select value={channels.toString()} onValueChange={(value) => setChannels(parseInt(value))}>
              <SelectTrigger className="col-span-3 bg-background border-border text-foreground">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="bg-background border-border">
                <SelectItem value="1" className="text-foreground">Mono (1)</SelectItem>
                <SelectItem value="2" className="text-foreground">Stereo (2)</SelectItem>
                <SelectItem value="6" className="text-foreground">5.1 Surround (6)</SelectItem>
                <SelectItem value="8" className="text-foreground">7.1 Surround (8)</SelectItem>
              </SelectContent>
            </Select>
          </div> */}
          {/* <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="sample-rate" className="text-right text-foreground">
              Sample Rate
            </Label>
            <Select value={sampleRate.toString()} onValueChange={(value) => setSampleRate(parseInt(value))}>
              <SelectTrigger className="col-span-3 bg-background border-border text-foreground">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="bg-background border-border">
                <SelectItem value="44100" className="text-foreground">44.1 kHz</SelectItem>
                <SelectItem value="48000" className="text-foreground">48 kHz</SelectItem>
                <SelectItem value="96000" className="text-foreground">96 kHz</SelectItem>
                <SelectItem value="192000" className="text-foreground">192 kHz</SelectItem>
              </SelectContent>
            </Select>
          </div> */}
        </div>
        <DialogFooter>
          <Button
            type='button'
            variant='outline'
            onClick={() => setOpen(false)}
            className='bg-background border-border text-foreground hover:bg-accent'
          >
            Cancel
          </Button>
          <Button
            type='submit'
            onClick={handleCreate}
            className='bg-purple-600 hover:bg-purple-700 dark:bg-purple-500 dark:hover:bg-purple-600 text-white'
          >
            Create Sink
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
